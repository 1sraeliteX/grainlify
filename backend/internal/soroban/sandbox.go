package soroban

import (
	"context"
	"fmt"
	"log/slog"
	"strings"
	"time"
)

// SandboxConfig holds configuration for sandbox shadow testing.
type SandboxConfig struct {
	Enabled                  bool
	EscrowSandboxContractID  string
	ProgramSandboxContractID string
	ShadowedOperations       []string // e.g. ["lock_funds", "release_funds", "refund"]
	SandboxSourceSecret      string   // Separate keypair to avoid tx_bad_seq with production
	MaxConcurrentShadows     int      // Bounds goroutine count (default: 10)
}

// SandboxManager mirrors selected contract operations to sandbox contract
// instances for testing new features against real-ish data flow. Shadow
// operations run asynchronously and never block or affect production calls.
type SandboxManager struct {
	config    SandboxConfig
	escrow    *EscrowContract
	program   *ProgramEscrowContract
	shadowOps map[string]bool
	sem       chan struct{}
}

// NewSandboxManager creates a SandboxManager with its own contract clients
// pointing at sandbox addresses and a separate TransactionBuilder. Returns an
// error if enabled but required configuration is missing.
func NewSandboxManager(client *Client, cfg SandboxConfig) (*SandboxManager, error) {
	if !cfg.Enabled {
		return &SandboxManager{config: cfg}, nil
	}

	if cfg.EscrowSandboxContractID == "" {
		return nil, fmt.Errorf("sandbox: SANDBOX_ESCROW_CONTRACT_ID is required when sandbox is enabled")
	}
	if cfg.ProgramSandboxContractID == "" {
		return nil, fmt.Errorf("sandbox: SANDBOX_PROGRAM_ESCROW_CONTRACT_ID is required when sandbox is enabled")
	}
	if cfg.SandboxSourceSecret == "" {
		return nil, fmt.Errorf("sandbox: SANDBOX_SOURCE_SECRET is required when sandbox is enabled")
	}

	maxConcurrent := cfg.MaxConcurrentShadows
	if maxConcurrent <= 0 {
		maxConcurrent = 10
	}

	// Create a separate TransactionBuilder with its own keypair so sandbox
	// transactions don't conflict with production sequence numbers.
	txBuilder, err := NewTransactionBuilder(client, cfg.SandboxSourceSecret, DefaultRetryConfig())
	if err != nil {
		return nil, fmt.Errorf("sandbox: failed to create transaction builder: %w", err)
	}

	// Build the operation lookup set.
	shadowOps := make(map[string]bool, len(cfg.ShadowedOperations))
	for _, op := range cfg.ShadowedOperations {
		op = strings.TrimSpace(op)
		if op != "" {
			shadowOps[op] = true
		}
	}

	slog.Info("sandbox mode enabled",
		"escrow_contract", cfg.EscrowSandboxContractID,
		"program_contract", cfg.ProgramSandboxContractID,
		"shadowed_operations", cfg.ShadowedOperations,
		"max_concurrent", maxConcurrent,
	)

	return &SandboxManager{
		config:    cfg,
		escrow:    NewEscrowContract(client, txBuilder, cfg.EscrowSandboxContractID),
		program:   NewProgramEscrowContract(client, txBuilder, cfg.ProgramSandboxContractID),
		shadowOps: shadowOps,
		sem:       make(chan struct{}, maxConcurrent),
	}, nil
}

// shouldShadow returns true if the given operation is configured for shadowing.
func (sm *SandboxManager) shouldShadow(operation string) bool {
	if !sm.config.Enabled {
		return false
	}
	return sm.shadowOps[operation]
}

// acquireSemaphore tries to acquire a semaphore slot without blocking.
// Returns false if the sandbox is at capacity.
func (sm *SandboxManager) acquireSemaphore() bool {
	select {
	case sm.sem <- struct{}{}:
		return true
	default:
		return false
	}
}

func (sm *SandboxManager) releaseSemaphore() {
	<-sm.sem
}

// logShadowResult emits a structured log entry for a completed shadow operation.
func logShadowResult(operation string, start time.Time, err error) {
	elapsed := time.Since(start)
	if err != nil {
		slog.Warn("sandbox shadow failed",
			"sandbox", true,
			"operation", operation,
			"duration_ms", elapsed.Milliseconds(),
			"error", err,
		)
		return
	}
	slog.Info("sandbox shadow succeeded",
		"sandbox", true,
		"operation", operation,
		"duration_ms", elapsed.Milliseconds(),
	)
}

// ShadowLockFunds mirrors a lock_funds call to the sandbox escrow contract.
func (sm *SandboxManager) ShadowLockFunds(ctx context.Context, depositor string, bountyID uint64, amount int64, deadline int64) {
	const op = "lock_funds"
	if !sm.shouldShadow(op) {
		return
	}
	if !sm.acquireSemaphore() {
		slog.Warn("sandbox shadow skipped: at capacity", "sandbox", true, "operation", op)
		return
	}

	// Detach from the HTTP request lifecycle so cancellation of the parent
	// context does not abort the shadow operation.
	shadowCtx := context.WithoutCancel(ctx)

	go func() {
		defer sm.releaseSemaphore()
		start := time.Now()
		_, err := sm.escrow.LockFunds(shadowCtx, depositor, bountyID, amount, deadline)
		logShadowResult(op, start, err)
	}()
}

// ShadowReleaseFunds mirrors a release_funds call to the sandbox escrow contract.
func (sm *SandboxManager) ShadowReleaseFunds(ctx context.Context, bountyID uint64, contributor string) {
	const op = "release_funds"
	if !sm.shouldShadow(op) {
		return
	}
	if !sm.acquireSemaphore() {
		slog.Warn("sandbox shadow skipped: at capacity", "sandbox", true, "operation", op)
		return
	}

	shadowCtx := context.WithoutCancel(ctx)

	go func() {
		defer sm.releaseSemaphore()
		start := time.Now()
		_, err := sm.escrow.ReleaseFunds(shadowCtx, bountyID, contributor)
		logShadowResult(op, start, err)
	}()
}

// ShadowRefund mirrors a refund call to the sandbox escrow contract.
func (sm *SandboxManager) ShadowRefund(ctx context.Context, bountyID uint64) {
	const op = "refund"
	if !sm.shouldShadow(op) {
		return
	}
	if !sm.acquireSemaphore() {
		slog.Warn("sandbox shadow skipped: at capacity", "sandbox", true, "operation", op)
		return
	}

	shadowCtx := context.WithoutCancel(ctx)

	go func() {
		defer sm.releaseSemaphore()
		start := time.Now()
		_, err := sm.escrow.Refund(shadowCtx, bountyID)
		logShadowResult(op, start, err)
	}()
}

// ShadowSinglePayout mirrors a single_payout call to the sandbox program contract.
func (sm *SandboxManager) ShadowSinglePayout(ctx context.Context, recipient string, amount int64) {
	const op = "single_payout"
	if !sm.shouldShadow(op) {
		return
	}
	if !sm.acquireSemaphore() {
		slog.Warn("sandbox shadow skipped: at capacity", "sandbox", true, "operation", op)
		return
	}

	shadowCtx := context.WithoutCancel(ctx)

	go func() {
		defer sm.releaseSemaphore()
		start := time.Now()
		_, err := sm.program.SinglePayout(shadowCtx, recipient, amount)
		logShadowResult(op, start, err)
	}()
}

// ShadowBatchPayout mirrors a batch_payout call to the sandbox program contract.
func (sm *SandboxManager) ShadowBatchPayout(ctx context.Context, payouts []PayoutItem) {
	const op = "batch_payout"
	if !sm.shouldShadow(op) {
		return
	}
	if !sm.acquireSemaphore() {
		slog.Warn("sandbox shadow skipped: at capacity", "sandbox", true, "operation", op)
		return
	}

	// Copy the slice to avoid races if the caller mutates it after returning.
	items := make([]PayoutItem, len(payouts))
	copy(items, payouts)

	shadowCtx := context.WithoutCancel(ctx)

	go func() {
		defer sm.releaseSemaphore()
		start := time.Now()
		_, err := sm.program.BatchPayout(shadowCtx, items)
		logShadowResult(op, start, err)
	}()
}
