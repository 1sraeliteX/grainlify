import sys

def resolve_file(filepath):
    with open(filepath, 'r') as f:
        lines = f.readlines()
        
    out = []
    i = 0
    while i < len(lines):
        if lines[i].startswith('<<<<<<<'):
            head_content = []
            i += 1
            while not lines[i].startswith('======='):
                head_content.append(lines[i])
                i += 1
            i += 1
            theirs_content = []
            while not lines[i].startswith('>>>>>>>'):
                theirs_content.append(lines[i])
                i += 1
            i += 1
            
            # Auto-resolve rules:
            # - DataKey combined
            if any('MaintenanceMode' in x for x in head_content):
                out.extend(head_content)
                out.extend(theirs_content)
            # - TWA combining BASIS_POINTS
            elif any('TWA_PERIOD' in x for x in head_content):
                out.extend(head_content)
                out.extend(theirs_content)
            # - ProgramData initialization (reference_hash)
            elif 'token_address: Address' in head_content[0] and 'token_address: Address' in theirs_content[0]:
                out.extend(theirs_content)
            # - Event PauseStateChanged struct vs tuple
            elif 'symbol_short!("lock")' in ''.join(head_content) or 'symbol_short!("release")' in ''.join(head_content) or 'symbol_short!("refund")' in ''.join(head_content):
                # use theirs_content because it has the new struct definition from upstream
                out.extend(theirs_content)
            # - ProgramReleaseSchedule vs tests... wait, some tests were stripped out or moved to bottom
            elif '#[test]' in ''.join(theirs_content):
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

resolve_file("contracts/program-escrow/src/lib.rs")
