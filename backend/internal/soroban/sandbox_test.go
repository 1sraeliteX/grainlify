package soroban

import (
	"context"
	"testing"
)

func TestShouldShadow_EnabledOperations(t *testing.T) {
	sm := &SandboxManager{
		config: SandboxConfig{Enabled: true},
		shadowOps: map[string]bool{
			"lock_funds":    true,
			"release_funds": true,
		},
		sem: make(chan struct{}, 10),
	}

	if !sm.shouldShadow("lock_funds") {
		t.Error("expected lock_funds to be shadowed")
	}
	if !sm.shouldShadow("release_funds") {
		t.Error("expected release_funds to be shadowed")
	}
	if sm.shouldShadow("refund") {
		t.Error("expected refund to NOT be shadowed")
	}
	if sm.shouldShadow("unknown_op") {
		t.Error("expected unknown_op to NOT be shadowed")
	}
}

func TestShouldShadow_DisabledGlobal(t *testing.T) {
	sm := &SandboxManager{
		config: SandboxConfig{Enabled: false},
		shadowOps: map[string]bool{
			"lock_funds": true,
		},
		sem: make(chan struct{}, 10),
	}

	if sm.shouldShadow("lock_funds") {
		t.Error("expected lock_funds to NOT be shadowed when sandbox is disabled")
	}
}

func TestShadowDisabledNoOp(t *testing.T) {
	// A disabled SandboxManager (created via NewSandboxManager with Enabled=false)
	// should have nil contract clients. Shadow methods must return immediately
	// without panicking.
	sm := &SandboxManager{
		config: SandboxConfig{Enabled: false},
		sem:    make(chan struct{}, 1),
	}

	// These must not panic even though escrow/program are nil.
	sm.ShadowLockFunds(context.Background(), "GABC", 1, 1000, 0)
	sm.ShadowReleaseFunds(context.Background(), 1, "GABC")
	sm.ShadowRefund(context.Background(), 1)
	sm.ShadowSinglePayout(context.Background(), "GABC", 500)
	sm.ShadowBatchPayout(context.Background(), []PayoutItem{{Recipient: "GABC", Amount: 100}})
}

func TestSemaphoreBound(t *testing.T) {
	sm := &SandboxManager{
		config: SandboxConfig{Enabled: true},
		shadowOps: map[string]bool{
			"lock_funds": true,
		},
		sem: make(chan struct{}, 2),
	}

	// Fill the semaphore manually.
	sm.sem <- struct{}{}
	sm.sem <- struct{}{}

	// acquireSemaphore should return false when full.
	if sm.acquireSemaphore() {
		t.Error("expected acquireSemaphore to return false when at capacity")
	}

	// Release one slot.
	sm.releaseSemaphore()
	if !sm.acquireSemaphore() {
		t.Error("expected acquireSemaphore to succeed after releasing a slot")
	}

	// Cleanup.
	sm.releaseSemaphore()
	sm.releaseSemaphore()
}

func TestShadowDetachedContext(t *testing.T) {
	// Verify that shouldShadow works correctly even when parent context is
	// already cancelled — the shouldShadow check itself doesn't depend on ctx.
	ctx, cancel := context.WithCancel(context.Background())
	cancel() // Cancel immediately.

	sm := &SandboxManager{
		config: SandboxConfig{Enabled: false},
		sem:    make(chan struct{}, 1),
	}

	// Should not panic with cancelled context; returns early because disabled.
	sm.ShadowLockFunds(ctx, "GABC", 1, 1000, 0)
}

func TestNewSandboxManager_Disabled(t *testing.T) {
	sm, err := NewSandboxManager(nil, SandboxConfig{Enabled: false})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if sm.config.Enabled {
		t.Error("expected sandbox to be disabled")
	}
}

func TestNewSandboxManager_MissingEscrowID(t *testing.T) {
	_, err := NewSandboxManager(nil, SandboxConfig{
		Enabled:                  true,
		EscrowSandboxContractID:  "",
		ProgramSandboxContractID: "CDEF",
		SandboxSourceSecret:      "SNOTAREALSECRET",
	})
	if err == nil {
		t.Error("expected error when escrow contract ID is missing")
	}
}

func TestNewSandboxManager_MissingProgramID(t *testing.T) {
	_, err := NewSandboxManager(nil, SandboxConfig{
		Enabled:                  true,
		EscrowSandboxContractID:  "CABC",
		ProgramSandboxContractID: "",
		SandboxSourceSecret:      "SNOTAREALSECRET",
	})
	if err == nil {
		t.Error("expected error when program contract ID is missing")
	}
}

func TestNewSandboxManager_MissingSourceSecret(t *testing.T) {
	_, err := NewSandboxManager(nil, SandboxConfig{
		Enabled:                  true,
		EscrowSandboxContractID:  "CABC",
		ProgramSandboxContractID: "CDEF",
		SandboxSourceSecret:      "",
	})
	if err == nil {
		t.Error("expected error when source secret is missing")
	}
}
