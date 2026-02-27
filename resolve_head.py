import re
import sys

files = [
    "contracts/program-escrow/src/test_full_lifecycle.rs",
    "contracts/program-escrow/src/test_pause.rs",
    "contracts/program-escrow/src/lib.rs",
    "contracts/program-escrow/src/test_circuit_breaker_audit.rs"
]

for filepath in files:
    with open(filepath, 'r') as f:
        content = f.read()

    # Replace conflict markers with HEAD content
    # pattern: <<<<<<< HEAD\n...\n=======\n...\n>>>>>>> (commit_hash or branch)
    pattern = re.compile(r'<<<<<<< HEAD\n(.*?)\n=======\n.*?\n>>>>>>> [^\n]+', re.DOTALL)
    new_content = pattern.sub(r'\1', content)
    
    with open(filepath, 'w') as f:
        f.write(new_content)
