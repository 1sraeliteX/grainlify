# Run CI steps locally (Format, Build, Test, Stellar Build)

From repo root, run the same steps as `.github/workflows/contracts-ci.yml`:

```bash
# 1. Format check (CI fails here if code is not formatted)
cd contracts/bounty_escrow/contracts/escrow
cargo fmt --check --all
# If it fails, fix with:
cargo fmt --all

# 2. Build for WASM (same target as CI)
cargo build --release --target wasm32v1-none

# 3. All tests
cargo test --verbose --lib

# 4. Invariant checker CI tests
cargo test --verbose --lib invariant_checker_ci

# 5. Stellar contract build (requires Stellar CLI)
stellar contract build --verbose
```

On Windows, `stellar contract build` may require Stellar CLI installed. Steps 1–4 are enough to catch most CI failures.
