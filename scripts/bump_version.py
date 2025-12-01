# Sapphillon
# SPDX-FileCopyrightText: 2025 Yuta Takahashi
# SPDX-License-Identifier: MPL-2.0 OR GPL-3.0-or-later

import sys
import re
import argparse

def bump_version(part):
    try:
        with open('Cargo.toml', 'r') as f:
            content = f.read()
    except FileNotFoundError:
        print("Error: Cargo.toml not found.")
        sys.exit(1)

    # Regex to find version in [workspace.package]
    # We look for [workspace.package] block start, then scan for version
    # Since TOML order can vary, but usually version is near the top of the block.
    # However, a robust way is to find the line `version = "..."` that appears after `[workspace.package]`
    # and before the next section `[` (or end of file).

    lines = content.splitlines()
    in_workspace_package = False
    new_lines = []
    version_found = False
    new_tag = ""

    for line in lines:
        stripped = line.strip()
        if stripped == '[workspace.package]':
            in_workspace_package = True
            new_lines.append(line)
            continue

        if in_workspace_package and stripped.startswith('[') and stripped.endswith(']'):
            in_workspace_package = False

        if in_workspace_package and stripped.startswith('version ='):
            match = re.match(r'version\s*=\s*"(\d+)\.(\d+)\.(\d+)"', stripped)
            if match:
                major, minor, patch = map(int, match.groups())

                if part == 'major':
                    major += 1
                    minor = 0
                    patch = 0
                elif part == 'minor':
                    minor += 1
                    patch = 0
                elif part == 'patch':
                    patch += 1

                new_version = f"{major}.{minor}.{patch}"
                new_tag = f"v{new_version}"
                new_lines.append(f'version = "{new_version}"')
                version_found = True
                in_workspace_package = False # Stop looking for version in this block (assuming only one)
                continue

        new_lines.append(line)

    if not version_found:
        # Fallback if regex didn't match perfectly or structure is different,
        # let's try a simpler approach if the file is small and structure is known.
        # But for now, report error.
        print("Error: Could not find 'version' in '[workspace.package]' section.")
        sys.exit(1)

    with open('Cargo.toml', 'w') as f:
        f.write('\n'.join(new_lines) + '\n')

    print(new_tag)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Bump Cargo.toml version')
    parser.add_argument('part', choices=['major', 'minor', 'patch'], help='Part of version to bump')
    args = parser.parse_args()

    bump_version(args.part)
