import os
import re
import sys

def fix_test_file(path):
    if not os.path.exists(path):
        print(f"File not found: {path}")
        return
        
    with open(path, 'r') as f:
        content = f.read()

    # Fix ProgramInitItem initializations missing fields
    def fix_init_item(match):
        inner = match.group(1)
        if 'creator:' not in inner:
            inner = inner.rstrip()
            if inner and not inner.endswith(','):
                inner += ','
            inner += "\n            creator: admin.clone(),"
        if 'initial_liquidity:' not in inner:
            inner = inner.rstrip()
            if inner and not inner.endswith(','):
                inner += ','
            inner += "\n            initial_liquidity: None,"
        return f"ProgramInitItem {{{inner}\n        }}"

    content = re.sub(r'ProgramInitItem\s*\{([^}]*)\}', fix_init_item, content)

    # Fix init_program calls (handling 3, 4, 5, 6-arg variants to make them all 6-arg for CLIENT)
    # The client signature is (program_id, authorized_payout_key, token_address, creator, initial_liquidity, reference_hash)
    
    # 3 args -> 6 args
    # Pattern: .init_program(&a, &b, &c)
    content = re.sub(r'(\.init_program\([^,)]+,[^,)]+,[^,)]+)\)', r'\1, &admin.clone(), &None, &None)', content)
    
    # 4 args -> 6 args (rare but just in case)
    content = re.sub(r'(\.init_program\([^,)]+,[^,)]+,[^,)]+,[^,)]+)\)', r'\1, &None, &None)', content)

    # 5 args -> 6 args
    content = re.sub(r'(\.init_program\([^,)]+,[^,)]+,[^,)]+,[^,)]+,[^,)]+)\)', r'\1, &None)', content)

    with open(path, 'w') as f:
        f.write(content)

# List of files to fix
files = [
    'contracts/program-escrow/src/test.rs',
    'contracts/program-escrow/src/test_pause.rs',
    'contracts/program-escrow/src/test_circuit_breaker_audit.rs',
    'contracts/program-escrow/src/test_full_lifecycle.rs',
    'contracts/program-escrow/src/test_granular_pause.rs',
    'contracts/program-escrow/src/reentrancy_tests.rs',
    'contracts/program-escrow/src/test_reputation.rs',
    'contracts/program-escrow/src/test_time_weighted_metrics.rs',
    'contracts/program-escrow/src/rbac_tests.rs',
    'contracts/program-escrow/src/test_lifecycle.rs',
    'contracts/program-escrow/src/test_metadata_tagging.rs',
    'contracts/program-escrow/src/test_claim_period_expiry_cancellation.rs',
    'contracts/program-escrow/src/test_payouts_splits.rs',
]

for f in files:
    fix_test_file(f)
