# Bounty Escrow Contract

## Dry-Run Simulation API

The escrow contract provides read-only dry-run entrypoints for previewing operations without mutating state. See [contracts/escrow/DRY_RUN_API.md](contracts/escrow/DRY_RUN_API.md) for full documentation.

- **dry_run_lock** – Simulate lock without transfers
- **dry_run_release** – Simulate release without transfers
- **dry_run_refund** – Simulate refund without transfers

All return `SimulationResult` with success/error_code/amount/resulting_status/remaining_amount. No authorization required.

## Metadata Constraints

The escrow metadata API enforces validation rules for human-readable tags like
`bounty_type`. See `contracts/escrow/METADATA_CONSTRAINTS.md` for the current
limits and guidance.

---

# Soroban Project

## Project Structure

This repository uses the recommended structure for a Soroban project:
```text
.
├── contracts
│   └── hello_world
│       ├── src
│       │   ├── lib.rs
│       │   └── test.rs
│       └── Cargo.toml
├── Cargo.toml
└── README.md
```

- New Soroban contracts can be put in `contracts`, each in their own directory. There is already a `hello_world` contract in there to get you started.
- If you initialized this project with any other example contracts via `--with-example`, those contracts will be in the `contracts` directory as well.
- Contracts should have their own `Cargo.toml` files that rely on the top-level `Cargo.toml` workspace for their dependencies.
- Frontend libraries can be added to the top-level directory as well. If you initialized this project with a frontend template via `--frontend-template` you will have those files already included.
