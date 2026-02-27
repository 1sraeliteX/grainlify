# On-Chain Risk Flags

This document describes the on-chain risk flag bitmask used for programs and escrows. These flags are intended as coarse-grained, admin-controlled signals for UIs and analytics.

## Flags (Bitmask)

- `RISK_FLAG_HIGH_RISK` (bit 0, value `1`): High risk. Use for entities that should be prominently warned about.
- `RISK_FLAG_UNDER_REVIEW` (bit 1, value `2`): Under review. Indicates active investigation or temporary uncertainty.
- `RISK_FLAG_RESTRICTED` (bit 2, value `4`): Restricted. Use for entities that should be de-emphasized or hidden by default.
- `RISK_FLAG_DEPRECATED` (bit 3, value `8`): Deprecated. Use for legacy or superseded entities.

Flags are additive; store and interpret them as a bitmask.

## Recommended Usage

- Treat flags as advisory signals rather than automatic enforcement.
- Prefer additive updates (set/clear specific bits) instead of overwriting the entire mask.
- Emit or consume events for auditability and off-chain indexing.
- Avoid using flags as sole evidence for punitive actions; pair with off-chain review.
