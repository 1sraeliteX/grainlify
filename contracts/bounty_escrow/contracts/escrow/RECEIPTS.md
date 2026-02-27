# Optional Require-Receipt for Critical Operations (Issue #677)

This document describes the receipt format and verification for critical operations (release and refund) in the bounty escrow contract.

## Overview

For accounting or legal workflows that need proof of settlement, the contract emits and stores a **receipt** for every release and refund. Each receipt is a committed proof of execution that can be used off-chain (e.g. from events) or verified on-chain via the `verify_receipt` view.

## Receipt Format

A receipt is represented by the `CriticalOperationReceipt` struct:

| Field       | Type                     | Description                                      |
|------------|---------------------------|--------------------------------------------------|
| `receipt_id` | `u64`                    | Unique monotonic id (assigned by the contract).  |
| `outcome`    | `CriticalOperationOutcome` | Either `Released` or `Refunded`.                 |
| `bounty_id`  | `u64`                    | Bounty that was released or refunded.            |
| `amount`     | `i128`                   | Amount transferred.                              |
| `party`      | `Address`                | Recipient (release) or refund-to address.       |
| `timestamp`  | `u64`                    | Ledger timestamp when the operation completed.   |

`CriticalOperationOutcome` is an enum: `Released` \| `Refunded`.

## When Receipts Are Issued

A receipt is **always** emitted and stored for:

- **Release**: `release_funds`, `release_with_capability`, `claim` (after claim execution), and each item in `batch_release_funds`.
- **Refund**: `refund` (both standard and approval-based flows).

No extra parameter is required; every critical operation produces one receipt.

## Events

Each receipt is published as an event with topic `("receipt", receipt_id)` and the receipt struct as data. Indexers and off-chain clients can:

1. Listen for `receipt` events to get proofs without calling the contract again.
2. Use the same payload for off-chain verification (e.g. signature over receipt data by a backend, if you add that layer).

## On-Chain Verification

- **`verify_receipt(env, receipt_id: u64) -> Option<CriticalOperationReceipt>`**

Returns the stored receipt for `receipt_id` if it exists. Use this to:

- Prove to another contract or script that a given release/refund completed.
- Re-check receipt fields (bounty_id, amount, party, timestamp) on-chain.

If the receipt was never stored (invalid id or pre–receipt deployment), the view returns `None`.

## Storage

- **ReceiptCounter** (persistent): Monotonic counter used to assign `receipt_id`.
- **Receipt(receipt_id)** (persistent): Maps each `receipt_id` to a `CriticalOperationReceipt`.

Receipts are stored so that `verify_receipt` can return them without relying on event history.

## Example Receipt (conceptual)

After a release of 1_000 units for bounty 42 to address `C...abc` at timestamp 1700000000, the first receipt might be:

```json
{
  "receipt_id": 1,
  "outcome": "Released",
  "bounty_id": 42,
  "amount": 1000,
  "party": "C...abc",
  "timestamp": 1700000000
}
```

Off-chain: use the event or a backend that signs this payload. On-chain: call `verify_receipt(1)` and compare the returned struct to expected values.

## Optional Signature (Future)

The issue allows an optional signature for receipts. The current implementation does not add a signature field; the contract only emits and stores the receipt. A future change could:

- Add an optional `signature: Option<BytesN<64>>` (or similar) to the receipt.
- Have a backend sign the receipt and submit it, or have the contract accept a pre-signed receipt for storage.

Verification would then be off-chain (signature check) or on-chain if the contract verifies the signature.
