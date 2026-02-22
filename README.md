# Sapphillon

## Overview

Sapphillon is an extensible workflow orchestration system developed in Rust. It enables flexible workflow automation through gRPC-based architecture and a plugin system.

## Key Features

### Core Features
- Workflow orchestration (JavaScript/TypeScript support)
- gRPC server (port 50051)
- SQLite database management (SeaORM)
- Extensible plugin system

### Built-in Plugins
- **fetch**: HTTP requests
- **filesystem**: File system operations
- **window**: Window management
- **exec**: Command execution
- **search**: File search

## Protocol Buffer Debug Tools

- evans
- buf

This repository generated from <https://github.com/Walkmana-25/rust-actions-example>

## Installation

### Dependencies
- Rust 2024 Edition
- Cargo
- SQLite

### Build Instructions
```bash
git clone https://github.com/Walkmana-25/Sapphillon.git
cd Sapphillon
cargo build --workspace --all-features
```

## Quick Start

### Starting the Server
```bash
# Default settings (in-memory DB)
cargo run -- start

# Debug mode (file-based DB)
cargo run -- --loglevel debug --db-url ./debug/sqlite.db start
```

### Command Line Options
| Option | Description | Default |
|-----------|------|------------|
| `--loglevel` | Log level | info |
| `--db-url` | Database URL | In-memory SQLite |
| `--ext-plugin-save-dir` | External plugin save directory | System temp directory |

## Project Structure

```
Sapphillon/
├── src/                    # Main source code
│   ├── main.rs            # Entry point
│   ├── server.rs          # gRPC server
│   └── services/          # gRPC services
├── entity/                # SeaORM entities
├── database/             # Database operations
├── migration/            # Migrations
├── plugins/              # Built-in plugins
└── docs/                 # Documentation
```

## Developer Information

For detailed development information, see [`DEVELOPERS.md`](DEVELOPERS.md).

### Makefile Targets
- `make rust_test`: Run tests
- `make rust_build`: Build
- `make rust_check_format`: Check code formatting
- `make rust_fix_format`: Fix code formatting
- `make migrate`: Run migrations
- `make run`: Run locally

### Development Workflow
```bash
# Initialize database
make gen_empty_db && make migrate && make entity_generate

# Start development server
make run
```

## Debug Workflow (Debug Build Only)

This feature is only enabled in debug builds. It periodically scans the `debug_workflow` directory for JavaScript files and automatically registers them to the database.

### Features

- **Periodic Scan**: Scans the `debug_workflow` directory every 10 seconds
- **Full Permissions**: Debug workflows are granted access to all plugins
- **Auto-Registration**: Detected JS files are automatically registered as workflows in the database

### Usage

1. Place JavaScript files in the `debug_workflow` directory
2. Run the application with a debug build (`cargo run`)
3. Workflows will be registered with the `[DEBUG]` prefix in the database

### Sample

```javascript
// debug_workflow/test.js
function workflow() {
    console.log("Debug workflow executed!");
    const result = fetch("https://api.example.com/data");
    console.log(result);
}
workflow();
```

> **Note**: This feature is only available in debug builds. It is disabled in release builds (`cargo build --release`).

## License

MPL-2.0 OR GPL-3.0-or-later

See [`LICENSE`](LICENSE), [`LICENSE-MPL`](LICENSE-MPL), and [`LICENSE-GPL`](LICENSE-GPL) for details.

## Copyright

© 2025 Yuta Takahashi

## Links

- [GitHub Repository](https://github.com/Walkmana-25/Sapphillon)
- [Developer Documentation](DEVELOPERS.md)
- [Test Documentation](src/tests/README.md)
