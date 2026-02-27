# Commit-Reveal Integration Guide

The commit-reveal pattern allows users to hide values until a future time, which is useful for sealed bids, confidential bounty submissions, or any workflow requiring fairness.

## How it Works

1. **Commit Phase**: The user hashes a `value` with a `salt` (`hash = sha256(value + salt)`) and submits this hash to the contract.
2. **Reveal Phase**: Later, the user provides the original `value` and `salt`. The contract verifies that `sha256(value + salt)` matches the previously stored hash.

## Usage in Soroban Contracts

### 1. Include the Library

Ensure your contract depends on `grainlify-core` or has access to the `commit_reveal` module.

```rust
use grainlify_core::{create_commitment, verify_reveal, Commitment, CommitRevealError};
```

### 2. Implementation Example

```rust
#[contracttype]
pub enum DataKey {
    Bid(Address),
}

#[contractimpl]
impl MyContract {
    pub fn commit_bid(env: Env, bidder: Address, hash: BytesN<32>, expiry: u64) {
        bidder.require_auth();
        
        let commitment = create_commitment(&env, bidder.clone(), hash, Some(expiry));
        env.storage().persistent().set(&DataKey::Bid(bidder), &commitment);
    }

    pub fn reveal_bid(env: Env, bidder: Address, value: Bytes, salt: Bytes) {
        let commitment: Commitment = env.storage().persistent().get(&DataKey::Bid(bidder))
            .expect("No bid found");
            
        // Verify the reveal
        verify_reveal(&env, &commitment, value, salt).expect("Reveal failed");
        
        // Process the revealed bid...
    }
}
```

## Best Practices

- **Salt Entropy**: Users should use a high-entropy random salt to prevent brute-force attacks on the hash.
- **Expiration**: Always set a reasonable expiry to prevent stale commitments from blocking state or being revealed too late.
- **Value Constraints**: Ensure the revealed value meets any contract-specific criteria after verification.
