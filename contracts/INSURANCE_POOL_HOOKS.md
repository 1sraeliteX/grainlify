# Insurance and Reserve Pool Integration via Hooks

## Overview

This document defines a standardized hook system that allows external insurance and reserve pool contracts to react to critical escrow events. Hooks enable composability without adding dependencies to core escrow logic.

## Architecture

### Event-Driven Model

```
┌─────────────────────────────────────────────────────────────┐
│                  Escrow Contract                             │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  1. Critical Event Occurs                                    │
│     (Dispute Opened, Large Release, etc.)                    │
│                                                               │
│  2. Emit Standardized Event                                  │
│     ↓                                                         │
│  3. Call Hook (Best-Effort)                                  │
│     ↓                                                         │
│     ├─→ Insurance Pool Contract                              │
│     ├─→ Reserve Pool Contract                                │
│     └─→ Custom Handler                                       │
│                                                               │
│  4. Continue Regardless of Hook Result                       │
│     (Core logic never blocked by hook failure)               │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### Key Design Principles

1. **Composability**: Hooks are optional and don't block core escrow operations
2. **Decoupling**: Insurance/reserve logic is completely separate from escrow
3. **Standardization**: Well-defined event schemas enable multiple integrations
4. **Safety**: Hook failures (e.g., contract bugs, insufficient funds) never block core flow
5. **Auditability**: All hook calls are emitted as events for off-chain monitoring
6. **Configurability**: Admin can enable/disable hooks per contract

## Event Schema

All hook events follow a versioned structure for forward compatibility:

```rust
#[contracttype]
#[derive(Clone, Debug)]
pub struct HookEvent {
    pub version: u32,           // Event version (v1, v2, etc.)
    pub event_type: Symbol,     // Type: dispute_opened, large_release, etc.
    pub bounty_id: u64,         // Bounty/program identifier
    pub amount: i128,           // Amount involved (if applicable)
    pub timestamp: u64,         // Event timestamp
    pub data: Map<Symbol, Val>, // Extensible additional data
}
```

### Event Types

#### 1. Dispute Opened
**Trigger**: When a dispute is initiated on an escrow bounty

```rust
#[contracttype]
#[derive(Clone, Debug)]
pub struct DisputeOpenedHook {
    pub version: u32,
    pub bounty_id: u64,
    pub disputer: Address,
    pub amount: i128,
    pub reason: DisputeReason,
    pub deadline: u64,
    pub timestamp: u64,
}
```

**When Insurance Pool Should React**:
- Assess coverage for the disputed amount
- Reserve funds if coverage is triggered
- Calculate potential exposure

---

#### 2. Large Release
**Trigger**: When a single release or batch release exceeds a threshold

```rust
#[contracttype]
#[derive(Clone, Debug)]
pub struct LargeReleaseHook {
    pub version: u32,
    pub bounty_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub release_count: u32, // Number of recipients in batch
    pub threshold: i128,    // Threshold that triggered hook
    pub timestamp: u64,
}
```

**When Reserve Pool Should React**:
- Verify reserve has sufficient liquidity
- Mark reserves as allocated
- Trigger additional checks or monitoring

---

#### 3. Refund
**Trigger**: When funds are refunded to depositor

```rust
#[contracttype]
#[derive(Clone, Debug)]
pub struct RefundHook {
    pub version: u32,
    pub bounty_id: u64,
    pub refund_to: Address,
    pub amount: i128,
    pub reason: RefundReason, // Expired, Cancelled, Disputed, etc.
    pub timestamp: u64,
}
```

**When Insurance Pool Should React**:
- Release claim on disputed amount (if applicable)
- Update coverage status

---

#### 4. Dispute Resolved
**Trigger**: When a dispute is resolved (approved or denied)

```rust
#[contracttype]
#[derive(Clone, Debug)]
pub struct DisputeResolvedHook {
    pub version: u32,
    pub bounty_id: u64,
    pub outcome: DisputeOutcome, // Approved, Denied, Partial
    pub amount_released: i128,
    pub amount_refunded: i128,
    pub timestamp: u64,
}
```

**When Insurance Pool Should React**:
- Settle claims based on outcome
- Update reserves
- Record claim history

---

## Hook Contract Interface

A hook contract must implement this interface:

```rust
/// Hook contract interface for insurance and reserve pool integration
#[contracttype]
pub enum HookEventType {
    DisputeOpened,
    LargeRelease,
    Refund,
    DisputeResolved,
}

#[contracttype]
pub struct HookCall {
    pub event_type: HookEventType,
    pub bounty_id: u64,
    pub amount: i128,
    pub timestamp: u64,
}

pub trait HookContract {
    /// Handle an escrow event
    /// Must return quickly and not consume excessive gas
    /// Failures should not panic; return error instead
    fn handle_hook(env: Env, call: HookCall) -> Result<(), String>;
}
```

### Implementation Guidelines

**Minimal Hook Example** (for Insurance Pool):

```rust
#[contract]
pub struct InsurancePoolHook;

#[contractimpl]
impl InsurancePoolHook {
    pub fn handle_hook(env: Env, call: HookCall) -> Result<(), String> {
        match call.event_type {
            HookEventType::DisputeOpened => {
                // 1. Check if amount exceeds pool's coverage limit
                let pool_data = Self::get_pool_data(&env)?;
                if call.amount > pool_data.coverage_limit {
                    return Err("Amount exceeds coverage limit".to_string());
                }
                
                // 2. Reserve coverage for this dispute
                Self::reserve_coverage(&env, call.bounty_id, call.amount)?;
                
                Ok(())
            }
            HookEventType::DisputeResolved => {
                // Settle the claim based on outcome
                Self::settle_claim(&env, call.bounty_id)?;
                Ok(())
            }
            _ => Ok(()), // Ignore other events
        }
    }
}
```

## Hook Management

### Enabling/Disabling Hooks

```rust
/// Set hook address (admin only)
pub fn set_hook_address(env: Env, admin: Address, hook_address: Option<Address>) {
    admin.require_auth();
    
    match hook_address {
        Some(addr) => {
            env.storage().instance().set(&DataKey::HookAddress, &addr);
            env.events().publish((symbol_short!("hook"),), ("enabled", addr));
        }
        None => {
            env.storage().instance().remove(&DataKey::HookAddress);
            env.events().publish((symbol_short!("hook"),), ("disabled", ()));
        }
    }
}

/// Get current hook address
pub fn get_hook_address(env: Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::HookAddress)
}
```

### Hook Execution Model

```rust
/// Internal helper - call hook if configured (best-effort)
fn call_hook_safe(
    env: &Env,
    event_type: HookEventType,
    bounty_id: u64,
    amount: i128,
) {
    // 1. Check if hook is configured
    let hook_addr = match env.storage().instance().get(&DataKey::HookAddress) {
        Some(addr) => addr,
        None => return, // No hook configured, continue normally
    };

    // 2. Prepare hook call
    let call = HookCall {
        event_type,
        bounty_id,
        amount,
        timestamp: env.ledger().timestamp(),
    };

    // 3. Attempt hook call (best-effort, errors logged but not propagated)
    let result = std::panic::catch_unwind(|| {
        let hook_client = HookContractClient::new(env, &hook_addr);
        hook_client.handle_hook(&call)
    });

    // 4. Log result as event
    match result {
        Ok(Ok(())) => {
            env.events().publish(
                (symbol_short!("hook_call"),),
                ("success", bounty_id, env.ledger().timestamp()),
            );
        }
        Ok(Err(err)) => {
            env.events().publish(
                (symbol_short!("hook_call"),),
                ("error", bounty_id, err.clone()),
            );
        }
        Err(_) => {
            env.events().publish(
                (symbol_short!("hook_call"),),
                ("panic", bounty_id),
            );
        }
    }
    
    // Core flow continues regardless of hook result
}
```

## Integration Examples

### Example 1: Insurance Pool with Dispute Coverage

```rust
pub struct InsurancePool {
    // Track coverage per bounty
    coverage: Map<u64, i128>, // bounty_id -> covered_amount
    coverage_limit: i128,
    reserve: i128,
}

impl InsurancePool {
    pub fn handle_hook(env: Env, call: HookCall) -> Result<(), String> {
        match call.event_type {
            HookEventType::DisputeOpened => {
                let mut pool = Self::load(&env)?;
                
                // Check reserve has enough liquidity
                if pool.reserve < call.amount {
                    return Err("Insufficient reserve".to_string());
                }
                
                // Reserve coverage
                pool.reserve -= call.amount;
                pool.coverage.insert(call.bounty_id, call.amount);
                pool.save(&env)?;
                Ok(())
            }
            HookEventType::DisputeResolved => {
                let mut pool = Self::load(&env)?;
                
                // Return reserved coverage to general reserve
                if let Some(amount) = pool.coverage.get(&call.bounty_id) {
                    pool.reserve += amount;
                    pool.coverage.remove(&call.bounty_id);
                }
                pool.save(&env)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
```

### Example 2: Reserve Pool with Large Release Monitoring

```rust
pub struct ReservePool {
    large_release_threshold: i128,
    monitored_releases: Vec<Release>,
}

impl ReservePool {
    pub fn handle_hook(env: Env, call: HookCall) -> Result<(), String> {
        match call.event_type {
            HookEventType::LargeRelease => {
                let mut pool = Self::load(&env)?;
                
                // Log the large release for monitoring
                pool.monitored_releases.push(Release {
                    bounty_id: call.bounty_id,
                    amount: call.amount,
                    timestamp: call.timestamp,
                });
                
                // Trigger additional checks if needed
                if call.amount > pool.large_release_threshold * 2 {
                    // Alert admin or trigger secondary verification
                    Self::alert_admins(&env, call.amount)?;
                }
                
                pool.save(&env)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
```

## Testing Hooks

### Mock Hook Contract for Tests

```rust
#[cfg(test)]
mod tests {
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct MockHookStats {
        pub calls_received: u32,
        pub last_event: HookEventType,
        pub last_amount: i128,
        pub should_fail: bool,
    }

    #[contract]
    pub struct MockHook;

    #[contractimpl]
    impl MockHook {
        pub fn handle_hook(env: Env, call: HookCall) -> Result<(), String> {
            let key = Symbol::new(&env, "mock_stats");
            let mut stats: MockHookStats = env
                .storage()
                .persistent()
                .get(&key)
                .unwrap_or(MockHookStats {
                    calls_received: 0,
                    last_event: call.event_type,
                    last_amount: 0,
                    should_fail: false,
                });

            stats.calls_received += 1;
            stats.last_event = call.event_type;
            stats.last_amount = call.amount;
            env.storage().persistent().set(&key, &stats);

            if stats.should_fail {
                Err("Mock hook failure".to_string())
            } else {
                Ok(())
            }
        }

        pub fn get_stats(env: Env) -> MockHookStats {
            let key = Symbol::new(&env, "mock_stats");
            env.storage()
                .persistent()
                .get(&key)
                .unwrap_or_default()
        }

        pub fn set_fail(env: Env, should_fail: bool) {
            // Implementation
        }
    }
}
```

### Test Pattern

```rust
#[test]
fn test_dispute_opened_calls_hook() {
    let env = Env::default();
    let admin = Address::random(&env);
    let hook = Address::random(&env);
    let depositor = Address::random(&env);
    
    // 1. Initialize escrow
    let escrow = BountyEscrowContract;
    escrow.init(&env, &admin, &token);
    
    // 2. Set hook
    escrow.set_hook_address(&env, &admin, &Some(hook));
    
    // 3. Open dispute
    escrow.open_dispute(&env, bounty_id, depositor.clone(), &reason);
    
    // 4. Verify mock hook received call
    let stats = MockHook::get_stats(&env);
    assert_eq!(stats.calls_received, 1);
    assert_eq!(stats.last_event, HookEventType::DisputeOpened);
    assert_eq!(stats.last_amount, amount);
}

#[test]
fn test_core_flow_continues_if_hook_fails() {
    let env = Env::default();
    let hook = Address::random(&env);
    
    // Set mock hook to always fail
    MockHook::set_fail(&env, true);
    
    // 1. Initialize and set hook
    escrow.set_hook_address(&env, &admin, &Some(hook));
    
    // 2. Perform operation
    escrow.open_dispute(&env, bounty_id, ...);
    
    // 3. Verify core operation succeeded despite hook failure
    let dispute = escrow.get_dispute(&env, bounty_id).expect("Dispute should exist");
    assert!(dispute.is_open);
}
```

## Event Audit Trail

Every hook call is emitted as an event for off-chain monitoring:

```rust
#[contracttype]
#[derive(Clone, Debug)]
pub struct HookCallEvent {
    pub bounty_id: u64,
    pub event_type: HookEventType,
    pub hook_address: Address,
    pub timestamp: u64,
    pub status: HookStatus, // Success, Error, Panic
    pub error_msg: Option<String>,
}

pub enum HookStatus {
    Success,
    Error,
    Panic,
}

// Emitted after every hook call
env.events().publish(
    (symbol_short!("hook_call"),),
    HookCallEvent { ... },
);
```

## Best Practices

### For Insurance Pool Implementers

1. **Fail Safely**: Never panic; return `Err` with descriptive message
2. **Be Quick**: Hook calls should complete in <100ms
3. **Validate Input**: Always validate `bounty_id`, `amount`, etc.
4. **Idempotent Operations**: Design hooks to be safely callable multiple times
5. **Monitor Reserves**: Always check sufficient liquidity before committing
6. **Log Events**: Emit your own events for settlement and claims

### For Escrow Contracts

1. **Call After Event**: Emit public event first, then call hook
2. **Use Best-Effort**: Never propagate hook errors up
3. **Timeout Protection**: Consider wrapping hook calls with timeout
4. **Rate Limiting**: If hooks are expensive, consider rate limiting by bounty_id
5. **Documentation**: Document exact conditions that trigger hooks

### For Integration Testing

1. **Mock Hooks**: Always provide a mock implementation for tests
2. **Failure Scenarios**: Test core flow with failing hooks
3. **Performance**: Measure hook call latency
4. **Idempotency**: Verify hook can be called multiple times safely

## Migration Guide

### Adding Hooks to Existing Escrow

1. **Add DataKey variant**:
```rust
#[contracttype]
pub enum DataKey {
    // ... existing variants
    HookAddress,
}
```

2. **Add hook getter/setter**:
```rust
pub fn set_hook_address(env: Env, admin: Address, hook: Option<Address>) {
    admin.require_auth();
    match hook {
        Some(addr) => env.storage().instance().set(&DataKey::HookAddress, &addr),
        None => env.storage().instance().remove(&DataKey::HookAddress),
    }
}
```

3. **Call hook at critical points**:
```rust
// After opening dispute
call_hook_safe(&env, HookEventType::DisputeOpened, bounty_id, amount);

// After large release
if amount > large_release_threshold {
    call_hook_safe(&env, HookEventType::LargeRelease, bounty_id, amount);
}
```

## FAQ

**Q: What if the hook address is not a contract?**
A: Hook calls will fail gracefully and be logged as errors. Core flow continues.

**Q: Can I change the hook address during operation?**
A: Yes, admin can enable/disable hooks anytime. New hook configuration applies to future events.

**Q: What's the maximum gas overhead of hooks?**
A: Hook calls add ~5-10k gas per critical operation. Design hooks to be minimal.

**Q: Can multiple contracts listen to the same events?**
A: Currently no - only one hook address is supported per escrow. Consider implementing a hook router contract to fan-out to multiple listeners.

**Q: How do I handle versioning if I need to change event schema?**
A: Include a `version` field in all hook events. Increment when adding required fields.

