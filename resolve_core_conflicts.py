import re
import glob

files = glob.glob("contracts/grainlify-core/src/*.rs")

for filepath in files:
    with open(filepath, 'r') as f:
        content = f.read()

    lines = content.split('\n')
    new_lines = []
    in_head = False
    in_remote = False

    for line in lines:
        if line.startswith('<<<<<<< HEAD'):
            in_head = True
            continue
        elif line.startswith('======='):
            if in_head:
                in_head = False
                in_remote = True
            else:
                new_lines.append(line)
            continue
        elif line.startswith('>>>>>>>'):
            if in_remote:
                in_remote = False
            else:
                new_lines.append(line)
            continue
            
        if in_remote:
            pass
        else:
            new_lines.append(line)

    with open(filepath, 'w') as f:
        f.write('\n'.join(new_lines))
