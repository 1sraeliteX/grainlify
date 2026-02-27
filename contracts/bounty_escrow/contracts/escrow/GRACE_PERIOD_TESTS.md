# Settlement Grace Period Tests Documentation

## Overview

This document outlines the comprehensive test suite for the configurable settlement grace periods feature. These tests validate that grace periods properly delay automatic settlement actions (refund and release) after deadlines, giving parties a final window to dispute or negotiate changes.

## Test Scenarios

### 1. Before Deadline Tests

**Test: `test_refund_fails_before_deadline`**

- **Setup**: Create escrow with deadline in future (1000 seconds)
- **Action**: Attempt refund before deadline passes
- **Expected Result**: ❌ FAILS with `DeadlineNotPassed` error
- **Validates**: Deadline enforcement is working

### 2. After Deadline Without Grace Period

**Test: `test_refund_allowed_after_deadline_no_grace`**

- **Setup**:
  - Create escrow with deadline 100 seconds in future
  - Grace period disabled (default: 0 seconds, enabled: false)
- **Action**:
  - Advance time to deadline
  - Attempt refund
- **Expected Result**: ✅ SUCCEEDS - escrow status changes to `Refunded`
- **Validates**: Legacy behavior when grace periods are disabled

### 3. During Grace Period Tests

**Test: `test_grace_period_blocks_refund_during_grace`**

- **Setup**:
  - Create escrow with deadline 100 seconds
  - Enable grace period of 500 seconds
- **Actions**:
  - Advance to deadline (deadline passed, but in grace period)
  - Attempt refund at deadline
  - Attempt refund at 250 seconds into grace
- **Expected Result**: ❌ FAILS both times - blocked by grace period
- **Validates**: Grace period prevents immediate auto-settlement

**Test: `test_schedule_release_blocks_during_grace`**

- **Setup**:
  - Create escrow with deadline 100 seconds
  - Create release schedule at deadline
  - Enable grace period of 500 seconds
- **Actions**:
  - Attempt release schedule automatic at deadline
  - Verify rejection during grace
- **Expected Result**: ❌ FAILS - blocked by grace period
- **Validates**: Grace period applies to scheduled releases too

### 4. After Grace Period Tests

**Test: `test_refund_allowed_after_grace_period`**

- **Setup**:
  - Create escrow with deadline 100 seconds
  - Enable grace period of 500 seconds
- **Actions**:
  - Advance past grace deadline (601 seconds total)
  - Attempt refund
- **Expected Result**: ✅ SUCCEEDS - escrow status changes to `Refunded`
- **Validates**: Settlement is allowed after grace period expires

### 5. Admin Override Tests

**Test: `test_admin_approval_bypasses_grace_period`**

- **Setup**:
  - Create escrow with deadline 100 seconds
  - Enable grace period of 500 seconds
- **Actions**:
  - Advance to 250 seconds (in grace period)
  - Admin approves refund
  - Attempt refund
- **Expected Result**: ✅ SUCCEEDS - admin approval bypasses grace
- **Validates**: Administrative refund approvals override grace periods

### 6. Configuration Tests

**Test: `test_grace_period_config_persistence`**

- **Setup**: Initial state
- **Actions**:
  - Get default config (should be disabled, 0 seconds)
  - Enable with 300 seconds
  - Verify persistence
  - Update to 600 seconds
  - Verify update
  - Disable
  - Verify disabled state
- **Expected Result**: ✅ All configuration changes persist correctly
- **Validates**: Configuration storage and retrieval

**Test: `test_non_admin_cannot_set_grace_period_config`**

- **Setup**: Create contract with admin
- **Action**: Non-admin attempts to set grace period config
- **Expected Result**: ❌ FAILS - unauthorized
- **Validates**: Access control on configuration

### 7. Edge Case Tests

**Test: `test_grace_period_zero_with_enabled_true`**

- **Setup**:
  - Create escrow
  - Set grace period to 0 seconds with enabled=true
- **Action**: Advance to deadline and attempt refund
- **Expected Result**: ✅ SUCCEEDS - grace_deadline = deadline + 0
- **Validates**: Boundary condition handling

**Test: `test_grace_period_large_values`**

- **Setup**:
  - Create escrow
  - Set grace period to 30 days (2,592,000 seconds)
  - Set deadline 100 seconds in future
- **Actions**:
  - Advance to deadline, attempt refund (fails - in grace)
  - Advance past grace period, attempt refund (succeeds)
- **Expected Result**: ✅ Large grace periods are handled correctly
- **Validates**: No integer overflow or boundary issues with large values

## Event Validation

### Events Emitted During Grace Period

1. **SettlementGracePeriodEntered**
   - Emitted when settlement action attempted during grace period
   - Contains: `bounty_id`, `grace_end_time`, `settlement_type` (refund/release), `timestamp`

2. **SettlementCompleted**
   - Emitted after successful settlement (after grace period expires)
   - Contains: `bounty_id`, `amount`, `recipient`, `settlement_type`, `timestamp`

## State Transitions

### Escrow Status During Grace Period

```
Locked → (deadline + grace) → Locked (no change)
  ↓
After grace period expires
  ↓
Released or Refunded
```

## Configuration Scenarios

| Scenario                | enabled | grace_period_seconds | Behavior                                          |
| ----------------------- | ------- | -------------------- | ------------------------------------------------- |
| Legacy (disabled)       | false   | 0                    | Settlement allowed immediately at deadline        |
| Short grace             | true    | 60                   | Settlement blocked for 60 seconds after deadline  |
| Standard grace          | true    | 300                  | Settlement blocked for 5 minutes after deadline   |
| Long grace              | true    | 2592000              | Settlement blocked for 30 days after deadline     |
| Zero grace with enabled | true    | 0                    | Settlement allowed at deadline (same as disabled) |

## Error Codes

- `DeadlineNotPassed` (6): Returned when settlement attempted before deadline + grace period
- `GraceperiodActive` (46): Returned when settlement attempted during grace period

## Critical Behaviors Validated

✅ Grace periods delay both refund and release operations
✅ Grace periods are configurable per contract (global setting)
✅ Default behavior is grace period disabled (backward compatible)
✅ Admin approvals bypass grace periods
✅ Grace period calculations handle large values correctly
✅ Grace periods emit appropriate events
✅ Only admin can configure grace periods
✅ Configuration changes persist across calls
✅ Settlement completes successfully after grace expires

## Test Coverage Summary

- **Total Test Cases**: 10
- **Permission Tests**: 1
- **Timing Tests**: 7
- **Configuration Tests**: 2
- **Edge Case Tests**: 2
- **Event Validation**: Implicit in all tests
- **Coverage**: Refund and scheduled release paths

## Execution Notes

- All tests use `mock_all_auths()` to simplify authorization
- Tests use relative timestamps (ledger time advancement)
- Grace periods are global contract configuration (not per-escrow)
- Admin bypass is tested for correctness
- Large grace period values are validated for safety

## Backward Compatibility

✅ Default configuration (enabled=false, grace_period_seconds=0) maintains legacy behavior
✅ Existing escrows created before grace periods work without changes
✅ Grace period feature is opt-in via admin configuration
