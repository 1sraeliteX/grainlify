# Time-Weighted Average (TWA) Metrics

## Overview

The program escrow contract exposes **time-weighted average** metrics over a **sliding window** to support richer on-chain analytics without heavy off-chain processing. Metrics are gas-efficient and bounded.

## Window

- **Strategy**: Fixed number of time buckets (rolling window).
- **Bucket count**: 24.
- **Period length**: 1 hour (`TWA_PERIOD_SECS = 3600`).
- **Window length**: 24 × 3600 = **86 400 seconds (24 hours)**.

Each bucket holds aggregates for one period. The current period is `ledger_timestamp / TWA_PERIOD_SECS`. Buckets are keyed by `period_id % 24`; when a new period starts, the bucket for that index is overwritten, so only the last 24 periods are kept.

## Metrics

| Metric | Description | Formula |
|--------|-------------|--------|
| **window_secs** | Length of the sliding window in seconds | `24 * 3600` |
| **avg_lock_size** | Average size of a lock in the window | `sum(lock_amounts) / lock_count` (0 if no locks) |
| **avg_settlement_time_secs** | Average time from last lock to payout, in seconds | `sum(settlement_time_secs) / settlement_count` (0 if no payouts) |
| **lock_count** | Number of lock operations in the window | Sum of `lock_count` in valid buckets |
| **settlement_count** | Number of payouts in the window | Sum of `settlement_count` in valid buckets |

**Settlement time** for a payout is defined as `payout_timestamp - last_lock_timestamp` (capped at 0). The contract stores `last_lock_timestamp` on each lock and uses it for every subsequent payout until the next lock.

## Storage (bounded)

- **TwaLastLock**: one `u64` (last lock timestamp).
- **TwaBucket(i)** for `i = 0..24`: each bucket is a `TwaBucket` with:
  - `period_id: u64`
  - `sum_lock_amount: i128`
  - `lock_count: u64`
  - `sum_settlement_time: u64`
  - `settlement_count: u64`

No unbounded lists; updates are O(1) per lock or payout.

## View

- **get_time_weighted_metrics(env)**  
  Returns a `TimeWeightedMetrics` with the fields above for the last 24 periods.

## Usage

Use these metrics to:

- Detect shifts in usage (e.g. average lock size or settlement time trending up or down).
- Compare recent activity to longer-term counters (e.g. from `get_program_aggregate_stats`) without replaying history off-chain.
