```markdown
# Developing Sapphillon

# Developing Sapphillon

## Makefile targets

This section documents common Makefile targets and their purpose.

- `rust_test`: Run Rust tests for the entire workspace with all features enabled (`cargo test --workspace --all-features`).
- `rust_build`: Build the Rust project for the whole workspace with all features (`cargo build --workspace --all-features`).
- `rust_check_format`: Check Rust formatting and run clippy. The Makefile runs `cargo fmt --all --check || true` and `cargo clippy --workspace || true`, so these checks are non-fatal to the `make` invocation.
- `rust_fix_format`: Fix Rust formatting and attempt automatic clippy fixes (`cargo fmt --all || true` and `cargo clippy --workspace --fix --allow-dirty || true`).
- `gen_empty_db`: Create an empty SQLite database file at `./db/sqlite.db` (creates the `db` directory and touches the file).
- `migrate_generate`: Generate a SeaORM migration. Usage: `make migrate_generate NAME=your_migration_name` (this calls `sea-orm-cli migrate generate $(NAME)`).
- `migrate`: Run SeaORM migrations against `sqlite://db/sqlite.db` (creates `db/sqlite.db` if missing and runs `sea-orm-cli migrate up -u "sqlite://db/sqlite.db"`).
- `entity_generate`: Generate SeaORM entity code from the configured database into `./entity/src/entity`.
- `run`: Run the Rust application for local/debug use. This target creates `./debug/plugins` and starts the app with debug logging, using `./debug/sqlite.db` as the DB and saving external plugins to `./debug/plugins` (invokes `cargo run -- --loglevel debug --db-url ./debug/sqlite.db --ext-plugin-save-dir ./debug/plugins start`).
- `grpcui`: Launch `grpcui` against the local gRPC server (runs `grpcui -plaintext localhost:50051`).

If you need to run a sequence of tasks (for example create the DB, run migrations, and generate entities), run the targets in order:

`make gen_empty_db && make migrate && make entity_generate`

## Notes

- The formatting/check targets in the Makefile are tolerant: `rust_check_format` runs `cargo fmt --all --check || true` and `cargo clippy --workspace || true` so they won't cause `make` to fail. `rust_fix_format` runs `cargo fmt --all || true` and `cargo clippy --workspace --fix --allow-dirty || true` to attempt automatic fixes without aborting the make run.
- `gen_empty_db`, `migrate`, and `entity_generate` work with `./db/sqlite.db` (the Makefile creates `./db` and touches the file if missing). The `run` target uses a separate debug DB at `./debug/sqlite.db` and stores runtime plugin files under `./debug/plugins`.

## Permissions System

### Wildcard Permission

The permission system supports a wildcard `plugin_function_id` of `*`. When a workflow is granted a permission with this `plugin_function_id`, it is allowed to bypass all permission checks for all plugins. This is useful for testing and for workflows that are trusted to have full access to the system.

## Internal Plugins Build System

### Overview

Sapphillon's build system provides automated discovery and registration of internal plugins from the `js_plugins` directory. This feature eliminates the need for manual plugin management and significantly simplifies the process of adding new internal plugins.

#### Key Features

- **Automatic Discovery**: Recursively scans the `js_plugins` directory to automatically detect plugins containing `package.js` files
- **Code Generation**: Generates the `src/internal_plugins.rs` file from discovered plugin information
- **Stable Ordering**: Plugins are sorted alphabetically and always returned in the same order
- **Build-time Regeneration**: The build script re-executes and regenerates code when changes are detected in the `js_plugins` directory

### Directory Structure

The internal plugin directory structure follows this pattern:

```
js_plugins/
├── {author_id}/
│   ├── {package_id}/
│   │   └── {version}/
│   │       ├── package.js      # Plugin information file
│   │       └── ...             # Other plugin files
│   └── ...
└── ...
```

#### Directory Structure Example

```
js_plugins/
├── app.sapphillon/
│   ├── example/
│   │   └── 1.0.0/
│   │       └── package.js
│   ├── plugin-a/
│   │   └── 1.0.0/
│   │       └── package.js
│   └── plugin-b/
│   │   └── 1.0.0/
│   │       └── package.js
└── other.author/
    └── plugin-c/
        └── 1.0.0/
            └── package.js
```

#### Path Pattern

- **Required Pattern**: `js_plugins/{author_id}/{package_id}/{version}/package.js`
- **author_id**: Plugin author identifier (e.g., `app.sapphillon`)
- **package_id**: Plugin identifier (e.g., `example`)
- **version**: Semantic versioning (e.g., `1.0.0`)

### Build Process Mechanism

#### Build Script Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    cargo build execution                      │
└──────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              build.rs main() function execution               │
└──────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│         discover_internal_plugins() call                       │
└──────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│    js_plugins directory existence check                       │
│    (warns and exits if not present)                           │
└──────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│    find_package_files() recursive search for package.js       │
└──────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│    extract_plugin_info() extracts plugin information          │
│    - Path structure validation                                │
│    - Version format validation                                │
└──────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│    generate_internal_plugins_file() code generation          │
│    - Creates src/internal_plugins.rs                          │
│    - Outputs plugins in alphabetical order                    │
└──────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│    cargo:rerun-if-changed=js_plugins output                  │
│    (monitors js_plugins directory changes)                    │
└──────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              Build continuation (Rust compilation)             │
└─────────────────────────────────────────────────────────────┘
```

### Automatically Generated Code

The build script automatically generates the `src/internal_plugins.rs` file. This file contains:

```rust
// Sapphillon
// SPDX-FileCopyrightText: 2025 Yuta Takahashi
// SPDX-License-Identifier: MPL-2.0 OR GPL-3.0-or-later
//
// This file is automatically generated by build.rs
// DO NOT EDIT THIS FILE MANUALLY
//
// This file contains information about internal plugins discovered
// from the js_plugins directory during build time.

use entity::entity::plugin_package::Model as PluginPackage;

/// Returns a list of internal plugins discovered at build time.
///
/// This function returns plugin information for all internal plugins
/// found in the js_plugins directory. The plugins are returned in
/// alphabetical order by package_id for stable ordering.
pub fn internal_plugins() -> Vec<PluginPackage> {
    let mut plugins = Vec::new();

    plugins.push(PluginPackage {
        package_id: "app.sapphillon/example/1.0.0".to_string(),
        package_name: "example".to_string(),
        package_version: "1.0.0".to_string(),
        description: None,
        plugin_store_url: None,
        internal_plugin: true,
        verified: true,
        deprecated: false,
        installed_at: None,
        updated_at: None,
    });

    // ... other plugins

    plugins
}
```

### Adding a New Plugin

#### Step 1: Create Directory Structure

Create the appropriate directory structure within the `js_plugins` directory:

```bash
# Create directory structure
mkdir -p js_plugins/app.sapphillon/my-plugin/1.0.0
```

#### Step 2: Create package.js File

Create a `package.js` file and define the plugin information:

```javascript
// Sapphillon
// SPDX-FileCopyrightText: 2025 Yuta Takahashi
// SPDX-License-Identifier: MPL-2.0 OR GPL-3.0-or-later

export default {
    name: "my-plugin",
    version: "1.0.0",
    description: "My custom internal plugin",
    functions: [
        {
            name: "myFunction",
            description: "A sample function",
            parameters: {
                type: "object",
                properties: {
                    input: {
                        type: "string",
                        description: "Input parameter"
                    }
                },
                required: ["input"]
            }
        }
    ]
};
```

#### Step 3: Implement the Plugin

Add necessary plugin implementation files as needed:

```bash
# Example: Create index.js file
touch js_plugins/app.sapphillon/my-plugin/1.0.0/index.js
```

#### Step 4: Run Build

Execute the build as usual, and the new plugin will be automatically discovered and registered:

```bash
cargo build
```

### Build Output Example

When building, discovered plugins are output as warning messages:

```bash
$ cargo build
   Compiling Sapphillon_Controller v0.1.0
warning: Discovered 4 internal plugin(s):
warning:   - app.sapphillon/example/1.0.0 (name: example, version: 1.0.0)
warning:   - app.sapphillon/plugin-a/1.0.0 (name: plugin-a, version: 1.0.0)
warning:   - app.sapphillon/plugin-b/1.0.0 (name: plugin-b, version: 1.0.0)
warning:   - other.author/plugin-c/1.0.0 (name: plugin-c, version: 1.0.0)
    Finished dev [unoptimized + debuginfo] target(s) in 2.45s
```

### Important Notes

1. **Do Not Edit Auto-generated Files**: `src/internal_plugins.rs` is automatically generated by the build script. Manual edits will be overwritten on the next build. The file contains a warning: `// DO NOT EDIT THIS FILE MANUALLY`.

2. **Strict Directory Structure**: The pattern `js_plugins/{author_id}/{package_id}/{version}/package.js` must be strictly followed. Files that don't match this pattern will be skipped with a warning.

3. **Version Format Validation**: Versions must start with a number (basic semantic versioning check). Invalid version formats will output a warning and skip that plugin.

4. **Windows Support Limitation**: Windows support is currently paused. The `build.rs` contains a compile error set with `#[cfg(target_os = "windows")]`.

5. **Alphabetical Sorting**: Plugins are sorted alphabetically by `package_id`. This is implemented using `BTreeMap` to ensure consistent ordering.

6. **Build-time Regeneration**: When changes are made to the `js_plugins` directory, the build script re-executes. This is controlled by the `cargo:rerun-if-changed=js_plugins` directive.

### Testing

The build system functionality is comprehensively tested in `tests/build_system.rs`:

```bash
# Run build system tests
cargo test --test build_system
```

Tests verify:
- Correct number of plugins
- Correct plugin information
- `internal_plugin` flag is `true`
- `plugin_store_url` is `None`
- Plugins are sorted alphabetically
- `verified` flag is `true`
- `deprecated` flag is `false`
- `description` is `None`
- `installed_at` and `updated_at` are `None`
- Correct `package_id` format
- Consistent results across multiple calls

### Related Files

| File | Description |
|------|-------------|
| `build.rs` | Build script (plugin discovery and code generation) |
| `src/internal_plugins.rs` | Auto-generated plugin information file |
| `tests/build_system.rs` | Build system integration tests |
| `js_plugins/` | Internal plugins directory |
| `docs/internal_plugin_build_system.md` | Detailed documentation in Japanese |

### Troubleshooting

#### Problem: New plugin not recognized

**Solution:**
1. Verify the directory structure is correct
2. Ensure the `package.js` file exists in the correct location
3. Verify the version starts with a number
4. Try a clean build with `cargo clean && cargo build`

#### Problem: Warnings appear during build

**Solution:**
1. Check the warning message to identify which file has issues
2. Verify the path structure and version format are correct
3. Fix or remove invalid files

#### Problem: Tests fail

**Solution:**
1. Verify `src/internal_plugins.rs` hasn't been manually edited
2. Check the `js_plugins` directory contents are correct
3. Try a clean build with `cargo clean && cargo test --test build_system`