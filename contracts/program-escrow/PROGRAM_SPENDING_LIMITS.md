# Program Spending Limits (Per-Window)

## Overview

Optional per-program, per-token spending limits cap the total amount released (payouts and schedule releases) within a configurable time window. This reduces blast radius from key compromise or bugs and supports operational risk policies.

## Behaviour

- **Scope**: Per program and per token (the program’s configured token).
- **Window**: Fixed-size windows in seconds (e.g. 86 400 for one day, 604 800 for one week).
- **Tracking**: Cumulative released amount in the current window; when the window expires, the next release starts a new window and the counter resets.
- **Enforcement**: Applied in:
  - `batch_payout`
  - `single_payout`
  - `release_prog_schedule_automatic`
  - `release_program_schedule_manual`
- **Optional**: If no limit is set or the limit is disabled, behaviour is unchanged (no cap).

## Configuration

- **Who**: Only the program’s **authorized payout key** can set or change the limit for that program.
- **How**: Call `set_program_spending_limit(program_id, window_size, max_amount, enabled)`.
- **Defaults**: No default limit; limits are off until explicitly configured.

### Parameters

| Parameter     | Type   | Description |
|---------------|--------|-------------|
| `program_id`  | String | Program ID. |
| `window_size` | u64    | Window length in seconds (e.g. 86400 = 1 day, 604800 = 1 week). |
| `max_amount`  | i128   | Max total amount (token’s smallest unit) that can be released in one window. Must be ≥ 0. |
| `enabled`     | bool   | If `false`, the limit is stored but not enforced until set to `true`. |

### Example (conceptual)

- Daily cap of 10 000 USDC (7 decimals):  
  `window_size = 86400`, `max_amount = 10_000_0000000`, `enabled = true`.
- Weekly cap of 50 000 USDC:  
  `window_size = 604800`, `max_amount = 50_000_0000000`, `enabled = true`.

## View Functions

- **`get_program_spending_limit(program_id)`**  
  Returns the current `ProgramSpendingConfig` for the program (and its token), if any.

- **`get_program_spending_state(program_id)`**  
  Returns the current `ProgramSpendingState` (window start and amount released in that window), if any.

## Accounting and Semantics

- **Precision**: Same units as the program’s token (smallest denomination). All releases in a window are summed; overflow on that sum is checked and panics if it would occur.
- **What counts**: For direct payouts, the full payout amount (including any fee taken from the same program balance) counts toward the window. For schedule releases, the schedule’s amount counts.
- **Window reset**: When `current_time - window_start >= window_size`, the next release is treated as the start of a new window: `window_start = current_time`, `amount_released = 0`, then the release amount is added.

## Events

When a release is rejected because the limit would be exceeded, the contract emits:

- Topic: `(limit, prog_spend)`
- Body: `(program_id, token, amount, new_total, max_amount, window_size)`

## Operational Usage

1. **Enable limits for high-risk or high-value programs**  
   Set a daily or weekly cap so that a compromised key or bug cannot drain more than the chosen amount in one window.

2. **Align with risk policies**  
   Use `window_size` and `max_amount` to match internal policies (e.g. “max X per day per program”).

3. **Monitor**  
   Use `get_program_spending_state` and `get_program_spending_limit` (and the `limit/prog_spend` event) to see usage and configuration.

4. **Disable without losing config**  
   Set `enabled = false` to turn off enforcement; the stored limit and window remain for re-enabling later.

## Testing

- **`test_program_spending_limit_enforced_for_batch_payout`**: Configures a limit, does one batch within the limit, then a second batch that would exceed the window limit and asserts the contract panics with “Program spending limit exceeded for current window”.
- **`test_program_spending_limit_resets_between_windows`**: Configures a short window, exhausts the limit in one window, advances time past the window, then asserts that a release of the same size succeeds in the new window.

## CI

Current GitHub Actions workflows (e.g. `contracts.yml`, `contracts-ci.yml`) build and test the **bounty_escrow** contract and run SDK smoke tests. The **program-escrow** contract (where spending limits are implemented) is not yet part of those workflows. To have CI cover program-escrow (and thus spending limits), add a job that builds and runs tests for `contracts/program-escrow`.
