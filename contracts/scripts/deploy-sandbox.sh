#!/bin/bash
# ==============================================================================
# Grainlify - Sandbox Contract Deployment Script
# ==============================================================================
# Deploys sandbox instances of escrow contracts for shadow testing.
# Uses the same WASM as production but deploys to separate contract addresses.
#
# USAGE:
#   ./scripts/deploy-sandbox.sh [options]
#
# OPTIONS:
#   -n, --network   Network to deploy to (testnet|mainnet) [default: testnet]
#   -i, --identity  Deployer identity name [default: sandbox-deployer]
#   --escrow-wasm   Path to escrow WASM [default: auto-detected]
#   --program-wasm  Path to program escrow WASM [default: auto-detected]
#   --dry-run       Simulate deployment without executing
#   -v, --verbose   Enable verbose output
#   -h, --help      Show this help message
#
# PREREQUISITES:
#   1. Create a sandbox deployer identity:
#      stellar keys generate --global sandbox-deployer
#      stellar keys fund sandbox-deployer --network testnet
#
#   2. Build contracts first:
#      cd contracts && cargo build --release --target wasm32-unknown-unknown
#
# OUTPUT:
#   Prints environment variables to add to your .env:
#     SANDBOX_ESCROW_CONTRACT_ID=<address>
#     SANDBOX_PROGRAM_ESCROW_CONTRACT_ID=<address>
#
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Source shared utilities
source "$SCRIPT_DIR/utils/common.sh"

# ------------------------------------------------------------------------------
# Defaults
# ------------------------------------------------------------------------------

NETWORK="testnet"
IDENTITY="sandbox-deployer"
ESCROW_WASM=""
PROGRAM_WASM=""
DRY_RUN="false"
VERBOSE="false"

# Auto-detect WASM paths
WASM_DIR="$PROJECT_ROOT/../soroban/target/wasm32-unknown-unknown/release"

# ------------------------------------------------------------------------------
# Usage
# ------------------------------------------------------------------------------

show_usage() {
    head -35 "$0" | grep -E "^#" | sed 's/^# \?//'
    exit 0
}

# ------------------------------------------------------------------------------
# Argument Parsing
# ------------------------------------------------------------------------------

while [[ $# -gt 0 ]]; do
    case "$1" in
        -n|--network)   NETWORK="$2"; shift 2 ;;
        -i|--identity)  IDENTITY="$2"; shift 2 ;;
        --escrow-wasm)  ESCROW_WASM="$2"; shift 2 ;;
        --program-wasm) PROGRAM_WASM="$2"; shift 2 ;;
        --dry-run)      DRY_RUN="true"; shift ;;
        -v|--verbose)   VERBOSE="true"; export VERBOSE; shift ;;
        -h|--help)      show_usage ;;
        *)
            log_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Auto-detect WASM files if not provided
if [[ -z "$ESCROW_WASM" ]]; then
    ESCROW_WASM="$WASM_DIR/escrow.wasm"
fi
if [[ -z "$PROGRAM_WASM" ]]; then
    PROGRAM_WASM="$WASM_DIR/program_escrow.wasm"
fi

# ------------------------------------------------------------------------------
# Validation
# ------------------------------------------------------------------------------

log_section "Sandbox Deployment"
log_info "Network: $NETWORK"
log_info "Identity: $IDENTITY"

if [[ ! -f "$ESCROW_WASM" ]]; then
    log_error "Escrow WASM not found: $ESCROW_WASM"
    log_error "Build contracts first: cargo build --release --target wasm32-unknown-unknown"
    exit 1
fi

if [[ ! -f "$PROGRAM_WASM" ]]; then
    log_warn "Program escrow WASM not found: $PROGRAM_WASM"
    log_warn "Only escrow sandbox will be deployed"
fi

# ------------------------------------------------------------------------------
# Deploy
# ------------------------------------------------------------------------------

SANDBOX_REGISTRY="$PROJECT_ROOT/deployments/sandbox-${NETWORK}.json"

log_section "Deploying Sandbox Escrow Contract"

DEPLOY_ARGS=(-n "$NETWORK" -i "$IDENTITY" -N "sandbox-escrow")
if [[ "$DRY_RUN" == "true" ]]; then
    DEPLOY_ARGS+=(--dry-run)
fi
if [[ "$VERBOSE" == "true" ]]; then
    DEPLOY_ARGS+=(-v)
fi

ESCROW_CONTRACT_ID=$("$SCRIPT_DIR/deploy.sh" "$ESCROW_WASM" "${DEPLOY_ARGS[@]}" | tail -1)

PROGRAM_CONTRACT_ID=""
if [[ -f "$PROGRAM_WASM" ]]; then
    log_section "Deploying Sandbox Program Escrow Contract"
    DEPLOY_ARGS_PROG=(-n "$NETWORK" -i "$IDENTITY" -N "sandbox-program-escrow")
    if [[ "$DRY_RUN" == "true" ]]; then
        DEPLOY_ARGS_PROG+=(--dry-run)
    fi
    if [[ "$VERBOSE" == "true" ]]; then
        DEPLOY_ARGS_PROG+=(-v)
    fi
    PROGRAM_CONTRACT_ID=$("$SCRIPT_DIR/deploy.sh" "$PROGRAM_WASM" "${DEPLOY_ARGS_PROG[@]}" | tail -1)
fi

# ------------------------------------------------------------------------------
# Output
# ------------------------------------------------------------------------------

log_section "Sandbox Deployment Complete"
echo ""
echo "Add these to your .env file:"
echo ""
echo "  SANDBOX_ENABLED=true"
echo "  SANDBOX_ESCROW_CONTRACT_ID=$ESCROW_CONTRACT_ID"
if [[ -n "$PROGRAM_CONTRACT_ID" ]]; then
    echo "  SANDBOX_PROGRAM_ESCROW_CONTRACT_ID=$PROGRAM_CONTRACT_ID"
fi
echo "  SANDBOX_SOURCE_SECRET=<your sandbox keypair secret>"
echo "  SANDBOX_SHADOWED_OPERATIONS=lock_funds,release_funds,refund,single_payout,batch_payout"
echo "  SANDBOX_MAX_CONCURRENT_SHADOWS=10"
echo ""
echo "Fund the sandbox source account:"
echo "  stellar keys fund <sandbox-source-identity> --network $NETWORK"
echo ""

log_success "Sandbox deployment script completed"
