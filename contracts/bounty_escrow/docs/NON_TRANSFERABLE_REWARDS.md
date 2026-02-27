# Non-Transferable (Soulbound) Reward Tokens

The bounty escrow supports marking an escrow as using **non-transferable** (soulbound) reward tokens. This aligns with reputation or credential use cases where the reward must not be transferred after claim.

## Contract behavior

- **Marking**: When locking funds, pass `non_transferable_rewards: Some(true)` to `lock_funds`, or set `non_transferable_rewards: true` on each item in `batch_lock_funds`.
- **Storage**: The flag is stored per bounty under `DataKey::NonTransferableRewards(bounty_id)` and is exposed via `get_non_transferable_rewards(bounty_id)`.
- **Events**: `FundsLocked` (and batch events) include `non_transferable_rewards: bool` for indexers and frontends.

## Token interface

The escrow uses the standard Soroban **TokenInterface** (e.g. SEP-41): it performs a single `transfer(from: contract, to: recipient, amount)` on release or claim. It does **not** require a separate “soulbound” token interface.

Non-transferability is enforced by the **token contract**:

- The escrow contract is an allowed spender/sender when it transfers to the recipient.
- After the recipient receives the tokens, the token contract must reject any further `transfer` (or equivalent) from that recipient, so the reward cannot be sold or forwarded.

So: use a token that implements the usual interface but restricts transfers from end-user accounts (e.g. only the escrow or a minter can move tokens to a user; users cannot transfer out).

## Claim and release logic

Claim and release logic is unchanged and compatible with soulbound tokens:

- **One transfer only**: The contract performs exactly one transfer from the escrow contract to the recipient. No further transfers are required.
- The recipient receives the reward; the token contract is responsible for blocking any subsequent transfers by the recipient.

## Usage

- **Single lock**: `lock_funds(depositor, bounty_id, amount, deadline, Some(true))`.
- **Batch lock**: In each `LockFundsItem`, set `non_transferable_rewards: true`.
- **Query**: `get_non_transferable_rewards(bounty_id)` returns `true` when the bounty is marked as non-transferable rewards.

Frontends and indexers can use the flag and event field to display or filter soulbound bounties and to warn users that the reward cannot be transferred after claim.
