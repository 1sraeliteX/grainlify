import sys
import os

def resolve_file(filepath):
    if not os.path.exists(filepath):
        return
    with open(filepath, 'r') as f:
        lines = f.readlines()
        
    out = []
    i = 0
    while i < len(lines):
        if lines[i].startswith('<<<<<<<'):
            head_content = []
            i += 1
            while i < len(lines) and not lines[i].startswith('======='):
                head_content.append(lines[i])
                i += 1
            if i < len(lines) and lines[i].startswith('======='):
                i += 1
            theirs_content = []
            while i < len(lines) and not lines[i].startswith('>>>>>>>'):
                theirs_content.append(lines[i])
                i += 1
            if i < len(lines) and lines[i].startswith('>>>>>>>'):
                i += 1
            
            # Combine logic
            combined = head_content + theirs_content
            head_str = "".join(head_content)
            theirs_str = "".join(theirs_content)
            
            if "MaintenanceMode" in head_str and "ReceiptCounter" in theirs_str:
                out.extend(head_content)
                out.extend(theirs_content)
            elif "TWA_PERIOD" in head_str:
                out.extend(head_content)
                out.extend(theirs_content)
            elif "authorized_payout_key" in theirs_str or "reference_hash" in theirs_str:
                out.extend(theirs_content)
                if "token_address: Address" in head_str and not "reference_hash" in head_str:
                    pass # We took the their side which has reference_hash
            elif "symbol_short!(\"lock\")" in head_str or "symbol_short!(\"release\")" in head_str or "symbol_short!(\"refund\")" in head_str:
                out.extend(theirs_content) # Use new struct format
            elif "set_maintenance_mode" in head_str or "is_maintenance_mode" in head_str or "Maintenance mode" in head_str:
                out.extend(head_content)
                out.extend(theirs_content)
            elif "get_program_release_schedule" in head_str:
                out.extend(head_content)
            elif "reentrancy_tests" in head_str or "test_time_weighted" in theirs_str:
                out.extend(head_content)
                out.extend(theirs_content)
            else:
                out.extend(head_content)
                out.extend(theirs_content)
        else:
            out.append(lines[i])
            i += 1
            
    with open(filepath, 'w') as f:
        f.writelines(out)

print("Resolving lib.rs...")
resolve_file("contracts/program-escrow/src/lib.rs")
resolve_file("contracts/program-escrow/src/test_pause.rs")
resolve_file("contracts/program-escrow/src/test_full_lifecycle.rs")
resolve_file("contracts/program-escrow/src/test_circuit_breaker_audit.rs")
print("Done")
