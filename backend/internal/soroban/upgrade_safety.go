package soroban

import (
	"context"
	"fmt"
	"time"

	"github.com/stellar/go/txnbuild"
	"github.com/stellar/go/xdr"
)

// UpgradeSafetyReport represents the result of an upgrade safety check
type UpgradeSafetyReport struct {
	IsSafe       bool              `json:"is_safe"`
	ChecksPassed uint32            `json:"checks_passed"`
	ChecksFailed uint32            `json:"checks_failed"`
	Warnings     []UpgradeWarning `json:"warnings"`
	Errors       []UpgradeError   `json:"errors"`
}

// UpgradeWarning represents a warning during safety check
type UpgradeWarning struct {
	Code    uint32 `json:"code"`
	Message string `json:"message"`
}

// UpgradeError represents an error during safety check
type UpgradeError struct {
	Code    uint32 `json:"code"`
	Message string `json:"message"`
}

// SafetyCheckCodes defines the codes for each safety check
var SafetyCheckCodes = map[uint32]string{
	1001: "Storage Layout Compatibility",
	1002: "Contract Initialization",
	1003: "Escrow State Consistency",
	1004: "Pending Claims Verification",
	1005: "Admin Authority",
	1006: "Token Configuration",
	1007: "Feature Flags Readiness",
	1008: "Reentrancy Lock",
	1009: "Version Compatibility",
	1010: "Balance Sanity",
}

// UpgradeSafetyClient provides methods for upgrade safety checks
type UpgradeSafetyClient struct {
	client        *Client
	contractAddr  string
}

// NewUpgradeSafetyClient creates a new upgrade safety client
func NewUpgradeSafetyClient(client *Client, contractAddress string) *UpgradeSafetyClient {
	return &UpgradeSafetyClient{
		client:       client,
		contractAddr: contractAddress,
	}
}

// SimulateUpgrade performs a dry-run of the upgrade safety checks
// This does not modify any state but validates all pre-conditions
func (u *UpgradeSafetyClient) SimulateUpgrade(ctx context.Context) (*UpgradeSafetyReport, error) {
	// Encode the contract address
	contractAddr, err := EncodeContractAddress(u.contractAddr)
	if err != nil {
		return nil, fmt.Errorf("invalid contract address: %w", err)
	}

	// Build the invoke host function for simulate_upgrade
	// The function takes no arguments
	op, err := BuildInvokeHostFunctionOp(contractAddr, "simulate_upgrade", []xdr.ScVal{})
	if err != nil {
		return nil, fmt.Errorf("failed to build operation: %w", err)
	}

	// Build and submit the transaction
	txBuilder := NewTransactionBuilder(u.client, u.contractAddr)
	result, err := txBuilder.BuildAndSubmit(ctx, []txnbuild.Operation{op})
	if err != nil {
		return nil, fmt.Errorf("failed to simulate upgrade: %w", err)
	}

	// Parse the result
	if len(result.Results) == 0 || result.Results[0] == nil {
		return nil, fmt.Errorf("no results returned from simulation")
	}

	// The result should contain the UpgradeSafetyReport
	// Parse the XDR return value
	var report UpgradeSafetyReport
	if err := xdr.Unmarshal(&report, result.Results[0].ReturnValue); err != nil {
		// If we can't parse, return a default report
		// This might happen if the contract hasn't implemented simulate_upgrade
		return &UpgradeSafetyReport{
			IsSafe:       false,
			ChecksPassed: 0,
			ChecksFailed: 1,
			Errors: []UpgradeError{
				{Code: 0, Message: "Contract does not support upgrade safety checks"},
			},
		}, nil
	}

	return &report, nil
}

// ValidateUpgrade performs the actual upgrade with safety checks
// This will fail if any safety check fails
func (u *UpgradeSafetyClient) ValidateUpgrade(ctx context.Context, newWasmHash uint32) error {
	// First, run safety simulation
	report, err := u.SimulateUpgrade(ctx)
	if err != nil {
		return fmt.Errorf("safety check failed: %w", err)
	}

	if !report.IsSafe {
		return fmt.Errorf("upgrade safety checks failed: %d errors, %d warnings",
			len(report.Errors), len(report.Warnings))
	}

	// Now perform the actual upgrade
	// Encode the contract address
	contractAddr, err := EncodeContractAddress(u.contractAddr)
	if err != nil {
		return fmt.Errorf("invalid contract address: %w", err)
	}

	// Encode the wasm hash as argument
	wasmHashVal, err := EncodeScValUint32(newWasmHash)
	if err != nil {
		return fmt.Errorf("failed to encode wasm hash: %w", err)
	}

	// Build the invoke host function for upgrade
	op, err := BuildInvokeHostFunctionOp(contractAddr, "upgrade", []xdr.ScVal{wasmHashVal})
	if err != nil {
		return fmt.Errorf("failed to build operation: %w", err)
	}

	// Build and submit the transaction
	txBuilder := NewTransactionBuilder(u.client, u.contractAddr)
	_, err = txBuilder.BuildAndSubmit(ctx, []txnbuild.Operation{op})
	if err != nil {
		return fmt.Errorf("failed to upgrade contract: %w", err)
	}

	return nil
}

// GetUpgradeSafetyStatus checks if safety checks are enabled
func (u *UpgradeSafetyClient) GetUpgradeSafetyStatus(ctx context.Context) (bool, error) {
	contractAddr, err := EncodeContractAddress(u.contractAddr)
	if err != nil {
		return false, fmt.Errorf("invalid contract address: %w", err)
	}

	op, err := BuildInvokeHostFunctionOp(contractAddr, "get_upgrade_safety_status", []xdr.ScVal{})
	if err != nil {
		return false, fmt.Errorf("failed to build operation: %w", err)
	}

	txBuilder := NewTransactionBuilder(u.client, u.contractAddr)
	result, err := txBuilder.BuildAndSubmit(ctx, []txnbuild.Operation{op})
	if err != nil {
		return false, fmt.Errorf("failed to get safety status: %w", err)
	}

	if len(result.Results) == 0 || result.Results[0] == nil {
		return false, fmt.Errorf("no results returned")
	}

	// Parse boolean result
	var enabled bool
	if err := xdr.Unmarshal(&enabled, result.Results[0].ReturnValue); err != nil {
		return false, fmt.Errorf("failed to parse result: %w", err)
	}

	return enabled, nil
}

// SetUpgradeSafety enables or disables safety checks
func (u *UpgradeSafetyClient) SetUpgradeSafety(ctx context.Context, enabled bool, adminKey *txnbuild.SimpleKey) error {
	contractAddr, err := EncodeContractAddress(u.contractAddr)
	if err != nil {
		return fmt.Errorf("invalid contract address: %w", err)
	}

	enabledVal, err := EncodeScValBool(enabled)
	if err != nil {
		return fmt.Errorf("failed to encode enabled: %w", err)
	}

	op, err := BuildInvokeHostFunctionOp(contractAddr, "set_upgrade_safety", []xdr.ScVal{enabledVal})
	if err != nil {
		return fmt.Errorf("failed to build operation: %w", err)
	}

	txBuilder := NewTransactionBuilderWithKey(u.client, u.contractAddr, adminKey)
	_, err = txBuilder.BuildAndSubmit(ctx, []txnbuild.Operation{op})
	if err != nil {
		return fmt.Errorf("failed to set safety status: %w", err)
	}

	return nil
}

// UpgradeSafetyConfig holds configuration for the upgrade safety system
type UpgradeSafetyConfig struct {
	// Timeout for safety check simulation
	SimulationTimeout time.Duration
	// Whether to require safety checks before upgrade
	RequireSafetyChecks bool
	// Maximum number of warnings allowed
	MaxWarnings uint32
}

// DefaultUpgradeSafetyConfig returns the default configuration
func DefaultUpgradeSafetyConfig() UpgradeSafetyConfig {
	return UpgradeSafetyConfig{
		SimulationTimeout:   30 * time.Second,
		RequireSafetyChecks: true,
		MaxWarnings:        0,
	}
}

// ValidateUpgradeWithConfig performs upgrade with custom configuration
func (u *UpgradeSafetyClient) ValidateUpgradeWithConfig(ctx context.Context, newWasmHash uint32, config UpgradeSafetyConfig) error {
	// Run safety simulation
	ctx, cancel := context.WithTimeout(ctx, config.SimulationTimeout)
	defer cancel()

	report, err := u.SimulateUpgrade(ctx)
	if err != nil {
		return fmt.Errorf("safety simulation failed: %w", err)
	}

	// Check if safety checks are required
	if config.RequireSafetyChecks && !report.IsSafe {
		return fmt.Errorf("upgrade rejected by safety checks: %d errors", len(report.Errors))
	}

	// Check warning threshold
	if report.ChecksFailed > 0 {
		return fmt.Errorf("upgrade has %d failed checks", report.ChecksFailed)
	}

	if report.ChecksPassed < 10 {
		return fmt.Errorf("incomplete safety check: only %d/10 checks passed", report.ChecksPassed)
	}

	// Perform the upgrade
	contractAddr, err := EncodeContractAddress(u.contractAddr)
	if err != nil {
		return fmt.Errorf("invalid contract address: %w", err)
	}

	wasmHashVal, err := EncodeScValUint32(newWasmHash)
	if err != nil {
		return fmt.Errorf("failed to encode wasm hash: %w", err)
	}

	op, err := BuildInvokeHostFunctionOp(contractAddr, "upgrade", []xdr.ScVal{wasmHashVal})
	if err != nil {
		return fmt.Errorf("failed to build operation: %w", err)
	}

	txBuilder := NewTransactionBuilder(u.client, u.contractAddr)
	_, err = txBuilder.BuildAndSubmit(ctx, []txnbuild.Operation{op})
	if err != nil {
		return fmt.Errorf("failed to upgrade contract: %w", err)
	}

	return nil
}

// FormatSafetyReport creates a human-readable string from the report
func FormatSafetyReport(report *UpgradeSafetyReport) string {
	var status string
	if report.IsSafe {
		status = "✓ SAFE TO UPGRADE"
	} else {
		status = "✗ UNSAFE TO UPGRADE"
	}

	output := fmt.Sprintf(`
══════════════════════════════════════════════════════════════════
  UPGRADE SAFETY REPORT
══════════════════════════════════════════════════════════════════
  Status: %s
  Checks Passed: %d
  Checks Failed: %d
══════════════════════════════════════════════════════════════════
`, status, report.ChecksPassed, report.ChecksFailed)

	if len(report.Errors) > 0 {
		output += "\nERRORS:\n"
		for _, err := range report.Errors {
			name := SafetyCheckCodes[err.Code]
			if name == "" {
				name = "Unknown"
			}
			output += fmt.Sprintf("  [%d] %s: %s\n", err.Code, name, err.Message)
		}
	}

	if len(report.Warnings) > 0 {
		output += "\nWARNINGS:\n"
		for _, warn := range report.Warnings {
			name := SafetyCheckCodes[warn.Code]
			if name == "" {
				name = "Unknown"
			}
			output += fmt.Sprintf("  [%d] %s: %s\n", warn.Code, name, warn.Message)
		}
	}

	output += "\n══════════════════════════════════════════════════════════════════\n"

	return output
}
