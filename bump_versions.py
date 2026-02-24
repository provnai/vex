
import os
import re

def bump_version(file_path):
    print(f"Bumping {file_path}")
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Use named groups or specific formatting to avoid \10 ambiguity
    pattern = r'(?P<prefix>\[package\]\n[^\[]*?version = ")0\.1\.6(?P<suffix>")'
    transformed = re.sub(pattern, r'\g<prefix>0.1.7\g<suffix>', content, flags=re.MULTILINE)
    
    if transformed != content:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(transformed)
        print("Success")
    else:
        print("No change needed or [package] version not found")

target_dirs = ['crates', 'examples']
for d in target_dirs:
    if not os.path.exists(d): continue
    for root, dirs, files in os.walk(d):
        if 'Cargo.toml' in files:
            bump_version(os.path.join(root, 'Cargo.toml'))
