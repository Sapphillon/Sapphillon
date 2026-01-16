// Sapphillon
// SPDX-FileCopyrightText: 2025 Yuta Takahashi
// SPDX-License-Identifier: MPL-2.0 OR GPL-3.0-or-later

//! Integration tests for external plugin installation and workflow execution.
//!
//! This module tests the complete flow of:
//! 1. Installing an external plugin from the filesystem
//! 2. Loading the plugin for use in workflows
//! 3. Executing workflows that call external plugin functions
//! 4. Verifying permission handling for external plugins
//!
//! ## Note on External Plugin Tests
//!
//! Tests that involve executing external plugins via `rsjs_bridge_core` or
//! `CoreWorkflowCode` with `CorePluginExternalPackage` require the external
//! plugin server binary to be built and accessible. These tests are marked
//! with `#[ignore]` and can be run with:
//!
//! ```bash
//! cargo test --test external_plugin_install_and_run -- --ignored
//! ```
//!
//! Before running ignored tests, ensure the external plugin server is built:
//! ```bash
//! cargo build --release -p ext_plugin
//! ```

use sapphillon_core::deno_core;
use sapphillon_core::ext_plugin::{RsJsBridgeArgs, RsJsBridgeReturns};
use sapphillon_core::extplugin_rsjs_bridge::rsjs_bridge_core;
use sapphillon_core::plugin::{CorePluginExternalPackage, PluginPackageTrait};
use sapphillon_core::runtime::OpStateWorkflowData;
use sapphillon_core::workflow::CoreWorkflowCode;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

// =============================================================================
// Test Fixture: Plugin JavaScript Code
// =============================================================================

/// JavaScript code for a simple math plugin used in tests.
const MATH_PLUGIN_JS: &str = r#"
globalThis.Sapphillon = {
    Package: {
        meta: {
            name: "math-plugin",
            version: "1.0.0",
            description: "A practical math plugin for integration testing",
            author_id: "com.sapphillon.test",
            package_id: "com.sapphillon.test.math-plugin"
        },
        functions: {
            add: {
                description: "Adds two numbers",
                permissions: [],
                parameters: [
                    { idx: 0, name: "a", type: "number", description: "First number" },
                    { idx: 1, name: "b", type: "number", description: "Second number" }
                ],
                returns: [{
                    idx: 0,
                    type: "number",
                    description: "Sum"
                }],
                handler: (a, b) => {
                    console.log(`[JS] Adding ${a} + ${b}`);
                    return a + b;
                }
            },
            process_data: {
                description: "Process a data object",
                permissions: [],
                parameters: [
                    { idx: 0, name: "data", type: "object", description: "Data object with value and multiplier" }
                ],
                returns: [{
                    idx: 0,
                    type: "object",
                    description: "Processed result"
                }],
                handler: (data) => {
                    console.log(`[JS] Processing data: ${JSON.stringify(data)}`);
                    return {
                        original: data.value,
                        result: data.value * data.multiplier,
                        timestamp: new Date().toISOString()
                    };
                }
            }
        }
    }
};
"#;

/// JavaScript code for an error-handling plugin used in tests.
const ERROR_PLUGIN_JS: &str = r#"
globalThis.Sapphillon = {
    Package: {
        meta: {
            name: "error-plugin",
            version: "1.0.0",
            description: "A plugin for testing error handling",
            author_id: "com.sapphillon.test",
            package_id: "com.sapphillon.test.error-plugin"
        },
        functions: {
            throw_immediate: {
                description: "Throws an immediate error",
                permissions: [],
                parameters: [],
                returns: [],
                handler: () => {
                    throw new Error("This is an immediate error");
                }
            },
            throw_async: {
                description: "Throws an async error",
                permissions: [],
                parameters: [],
                returns: [],
                handler: async () => {
                    await new Promise(resolve => setTimeout(resolve, 10));
                    throw new Error("This is an async error");
                }
            },
            async_success: {
                description: "Returns a value asynchronously",
                permissions: [],
                parameters: [
                    { idx: 0, name: "value", type: "string", description: "Value to transform" }
                ],
                returns: [{
                    idx: 0,
                    type: "string",
                    description: "Transformed value"
                }],
                handler: async (value) => {
                    await new Promise(resolve => setTimeout(resolve, 10));
                    return `async: ${value}`;
                }
            },
            return_null: {
                description: "Returns null",
                permissions: [],
                parameters: [],
                returns: [],
                handler: () => {
                    return null;
                }
            },
            no_op: {
                description: "Does nothing",
                permissions: [],
                parameters: [],
                returns: [],
                handler: () => {
                    // no-op
                }
            }
        }
    }
};
"#;

/// JavaScript code for a plugin that requires filesystem permissions.
const FILE_PLUGIN_JS: &str = r#"
globalThis.Sapphillon = {
    Package: {
        meta: {
            name: "file-plugin",
            version: "1.0.0",
            description: "A plugin for testing filesystem permissions",
            author_id: "com.sapphillon.test",
            package_id: "com.sapphillon.test.file-plugin"
        },
        functions: {
            read_file: {
                description: "Reads a file (requires FilesystemRead permission)",
                permissions: [{ 
                    type: "FilesystemRead", 
                    level: 1,
                    display_name: "Filesystem Read",
                    description: "Read access to filesystem",
                    resource: []
                }],
                parameters: [
                    { idx: 0, name: "path", type: "string", description: "File path to read" }
                ],
                returns: [{
                    idx: 0,
                    type: "string",
                    description: "File contents"
                }],
                handler: async (path) => {
                    const content = await Deno.readTextFile(path);
                    return content;
                }
            },
            simple_function: {
                description: "A simple function that requires no permissions",
                permissions: [],
                parameters: [
                    { idx: 0, name: "message", type: "string", description: "Message to echo" }
                ],
                returns: [{
                    idx: 0,
                    type: "string",
                    description: "Echoed message"
                }],
                handler: (message) => {
                    return `Echo: ${message}`;
                }
            }
        }
    }
};
"#;

// =============================================================================
// Helper Functions
// =============================================================================

/// Creates a temporary directory with plugin structure and returns the path.
///
/// This simulates what `install_ext_plugin` does: creating the directory
/// structure `{save_dir}/{author_id}/{package_id}/{version}/package.js`
fn create_temp_plugin(
    author_id: &str,
    package_id: &str,
    version: &str,
    package_js_content: &str,
) -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let plugin_dir = temp_dir
        .path()
        .join(author_id)
        .join(package_id)
        .join(version);
    std::fs::create_dir_all(&plugin_dir).expect("Failed to create plugin dir");

    let package_js_path = plugin_dir.join("package.js");
    std::fs::write(&package_js_path, package_js_content).expect("Failed to write package.js");

    (temp_dir, package_js_path)
}

/// Creates OpState with workflow data containing the test external package.
/// Returns the OpState and the tokio Runtime to keep the runtime alive.
#[allow(dead_code)]
fn create_opstate_with_package(
    package_js: &str,
    package_name: &str,
    author_id: &str,
) -> (deno_core::OpState, tokio::runtime::Runtime) {
    let mut op_state = deno_core::OpState::new(None);

    // Create the external package
    let package_id = format!("{author_id}.{package_name}");

    let package = CorePluginExternalPackage::new(
        package_id,
        package_name.to_string(),
        vec![], // functions list not needed for this test
        package_js.to_string(),
    );

    // Create OpStateWorkflowData with the external package
    let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
    let workflow_data = OpStateWorkflowData::new(
        "test_workflow",
        false,
        None,
        None,
        tokio_runtime.handle().clone(),
        vec![Arc::new(package)],
        None,
        None,
    );

    // Put workflow data into OpState
    op_state.put(Arc::new(Mutex::new(workflow_data)));

    (op_state, tokio_runtime)
}

/// Helper function to scan a directory for installed plugins.
/// Returns a vector of plugin IDs in the format "author/package/version".
fn scan_plugin_directory(save_dir: &str) -> Vec<String> {
    use std::fs;
    use std::path::Path;

    let mut plugin_ids = Vec::new();
    let base_path = Path::new(save_dir);

    if !base_path.exists() {
        return plugin_ids;
    }

    // Traverse: author-id/package-id/version/package.js
    if let Ok(author_entries) = fs::read_dir(base_path) {
        for author_entry in author_entries.flatten() {
            if !author_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let author_id = author_entry.file_name().to_string_lossy().to_string();

            if let Ok(package_entries) = fs::read_dir(author_entry.path()) {
                for package_entry in package_entries.flatten() {
                    if !package_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        continue;
                    }
                    let package_id = package_entry.file_name().to_string_lossy().to_string();

                    if let Ok(version_entries) = fs::read_dir(package_entry.path()) {
                        for version_entry in version_entries.flatten() {
                            if !version_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                                continue;
                            }
                            let version = version_entry.file_name().to_string_lossy().to_string();

                            // Check if package.js exists
                            let package_js = version_entry.path().join("package.js");
                            if package_js.exists() {
                                plugin_ids.push(format!("{author_id}/{package_id}/{version}"));
                            }
                        }
                    }
                }
            }
        }
    }

    plugin_ids
}

// =============================================================================
// Integration Tests: Plugin Installation and Filesystem Operations
// =============================================================================

/// Integration Test: Plugin Installation Creates Correct Directory Structure
///
/// **Purpose:**
/// Verify that installing an external plugin creates the correct directory
/// structure and writes the package.js file to the expected location.
///
/// **Flow:**
/// 1. Create a temporary directory to simulate the plugin save directory.
/// 2. Create a mock plugin file with the expected directory structure.
/// 3. Verify the file exists at the expected path.
/// 4. Verify the file content matches the original.
#[test]
fn test_plugin_installation_creates_directory_structure() {
    let author_id = "com.test.author";
    let package_id = "my-plugin";
    let version = "1.0.0";
    let plugin_content = MATH_PLUGIN_JS;

    let (temp_dir, package_js_path) =
        create_temp_plugin(author_id, package_id, version, plugin_content);

    // Verify the file was created
    assert!(
        package_js_path.exists(),
        "package.js should exist at {:?}",
        package_js_path
    );

    // Verify the content
    let content = std::fs::read_to_string(&package_js_path).expect("Failed to read package.js");
    assert_eq!(content, plugin_content);

    // Verify the directory structure
    let expected_dir = temp_dir.path().join(author_id).join(package_id).join(version);
    assert!(expected_dir.exists(), "Plugin directory should exist");
    assert!(expected_dir.is_dir(), "Plugin path should be a directory");
}

/// Integration Test: Plugin Scan Finds Installed Plugins
///
/// **Purpose:**
/// Verify that the plugin scanning functionality can find plugins installed
/// in the expected directory structure.
///
/// **Flow:**
/// 1. Create multiple mock plugins in a temporary directory.
/// 2. Scan the directory for plugins.
/// 3. Verify all plugins are found.
#[test]
fn test_plugin_scan_finds_installed_plugins() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create plugin 1
    let plugin1_dir = temp_dir.path().join("author1/plugin1/1.0.0");
    std::fs::create_dir_all(&plugin1_dir).expect("Failed to create plugin1 dir");
    std::fs::write(plugin1_dir.join("package.js"), MATH_PLUGIN_JS)
        .expect("Failed to write plugin1");

    // Create plugin 2
    let plugin2_dir = temp_dir.path().join("author2/plugin2/2.0.0");
    std::fs::create_dir_all(&plugin2_dir).expect("Failed to create plugin2 dir");
    std::fs::write(plugin2_dir.join("package.js"), ERROR_PLUGIN_JS)
        .expect("Failed to write plugin2");

    // Create incomplete plugin (no package.js)
    let incomplete_dir = temp_dir.path().join("author3/incomplete/1.0.0");
    std::fs::create_dir_all(&incomplete_dir).expect("Failed to create incomplete dir");

    // Scan for plugins
    let save_dir = temp_dir.path().to_string_lossy().to_string();
    let plugins = scan_plugin_directory(&save_dir);

    assert_eq!(plugins.len(), 2, "Should find exactly 2 valid plugins");
    assert!(
        plugins.contains(&"author1/plugin1/1.0.0".to_string()),
        "Should find plugin1"
    );
    assert!(
        plugins.contains(&"author2/plugin2/2.0.0".to_string()),
        "Should find plugin2"
    );
    assert!(
        !plugins.contains(&"author3/incomplete/1.0.0".to_string()),
        "Should not find incomplete plugin"
    );
}

/// Integration Test: Plugin Content Validation
///
/// **Purpose:**
/// Verify that plugin JavaScript content can be read and parsed correctly.
#[test]
fn test_plugin_content_validation() {
    let (_temp_dir, package_js_path) =
        create_temp_plugin("com.test", "test-plugin", "1.0.0", MATH_PLUGIN_JS);

    let content = std::fs::read_to_string(&package_js_path).expect("Failed to read package.js");

    // Verify the content contains expected structure
    assert!(
        content.contains("globalThis.Sapphillon"),
        "Should contain globalThis.Sapphillon"
    );
    assert!(content.contains("Package"), "Should contain Package");
    assert!(content.contains("functions"), "Should contain functions");
    assert!(content.contains("add"), "Should contain add function");
    assert!(
        content.contains("process_data"),
        "Should contain process_data function"
    );
}

/// Integration Test: Multiple Plugin Versions
///
/// **Purpose:**
/// Verify that multiple versions of the same plugin can coexist.
#[test]
fn test_multiple_plugin_versions() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create version 1.0.0
    let v1_dir = temp_dir.path().join("author/plugin/1.0.0");
    std::fs::create_dir_all(&v1_dir).expect("Failed to create v1 dir");
    std::fs::write(v1_dir.join("package.js"), "// v1.0.0").expect("Failed to write v1");

    // Create version 2.0.0
    let v2_dir = temp_dir.path().join("author/plugin/2.0.0");
    std::fs::create_dir_all(&v2_dir).expect("Failed to create v2 dir");
    std::fs::write(v2_dir.join("package.js"), "// v2.0.0").expect("Failed to write v2");

    // Create version 3.0.0-beta
    let v3_dir = temp_dir.path().join("author/plugin/3.0.0-beta");
    std::fs::create_dir_all(&v3_dir).expect("Failed to create v3 dir");
    std::fs::write(v3_dir.join("package.js"), "// v3.0.0-beta").expect("Failed to write v3");

    // Scan and verify all versions are found
    let save_dir = temp_dir.path().to_string_lossy().to_string();
    let plugins = scan_plugin_directory(&save_dir);

    assert_eq!(plugins.len(), 3, "Should find 3 plugin versions");
    assert!(plugins.contains(&"author/plugin/1.0.0".to_string()));
    assert!(plugins.contains(&"author/plugin/2.0.0".to_string()));
    assert!(plugins.contains(&"author/plugin/3.0.0-beta".to_string()));
}

/// Integration Test: Plugin Overwrite
///
/// **Purpose:**
/// Verify that a plugin can be overwritten with new content.
#[test]
fn test_plugin_overwrite() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let plugin_dir = temp_dir.path().join("author/plugin/1.0.0");
    std::fs::create_dir_all(&plugin_dir).expect("Failed to create dir");

    let package_js_path = plugin_dir.join("package.js");

    // Write initial content
    std::fs::write(&package_js_path, "// initial content").expect("Failed to write initial");
    assert_eq!(
        std::fs::read_to_string(&package_js_path).unwrap(),
        "// initial content"
    );

    // Overwrite with new content
    std::fs::write(&package_js_path, "// updated content").expect("Failed to write updated");
    assert_eq!(
        std::fs::read_to_string(&package_js_path).unwrap(),
        "// updated content"
    );
}

/// Integration Test: Plugin Removal
///
/// **Purpose:**
/// Verify that a plugin can be removed and is no longer detected.
#[test]
fn test_plugin_removal() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create plugin
    let plugin_dir = temp_dir.path().join("author/plugin/1.0.0");
    std::fs::create_dir_all(&plugin_dir).expect("Failed to create dir");
    std::fs::write(plugin_dir.join("package.js"), MATH_PLUGIN_JS).expect("Failed to write");

    // Verify plugin exists
    let save_dir = temp_dir.path().to_string_lossy().to_string();
    let plugins = scan_plugin_directory(&save_dir);
    assert_eq!(plugins.len(), 1);

    // Remove plugin
    std::fs::remove_dir_all(&plugin_dir).expect("Failed to remove plugin");

    // Verify plugin is gone
    let plugins_after = scan_plugin_directory(&save_dir);
    assert_eq!(plugins_after.len(), 0, "Plugin should be removed");
}

// =============================================================================
// Integration Tests: Bridge Function Execution
// These tests require the external plugin server to be running.
// Run with: cargo test --test external_plugin_install_and_run -- --ignored
// =============================================================================

/// Integration Test: Basic Function Execution via rsjs_bridge_core
///
/// **Purpose:**
/// Verify that the `rsjs_bridge_core` can correctly execute a simple function (`add`)
/// from an external plugin package.
///
/// **Note:** This test requires the external plugin server binary to be built.
/// Run with `cargo test -- --ignored` after building the ext_plugin server.
#[test]
#[ignore = "Requires external plugin server binary (cargo build -p ext_plugin)"]
fn test_bridge_basic_function_execution() {
    let (mut op_state, _tokio_rt) =
        create_opstate_with_package(MATH_PLUGIN_JS, "math-plugin", "com.sapphillon.test");

    let args = RsJsBridgeArgs {
        func_name: "add".to_string(),
        args: vec![("a".to_string(), json!(10)), ("b".to_string(), json!(20))]
            .into_iter()
            .collect(),
    };
    let args_json = args.to_string().unwrap();

    let result = rsjs_bridge_core(&mut op_state, &args_json, "com.sapphillon.test.math-plugin");
    assert!(
        result.is_ok(),
        "Bridge execution failed: {:?}",
        result.err()
    );

    let result_json = result.unwrap();
    let returns = RsJsBridgeReturns::new_from_str(&result_json).expect("Failed to parse returns");

    assert_eq!(returns.args.get("result"), Some(&json!(30)));
}

/// Integration Test: Complex Object Handling via rsjs_bridge_core
///
/// **Note:** This test requires the external plugin server binary.
#[test]
#[ignore = "Requires external plugin server binary"]
fn test_bridge_complex_object_handling() {
    let (mut op_state, _tokio_rt) =
        create_opstate_with_package(MATH_PLUGIN_JS, "math-plugin", "com.sapphillon.test");

    let input_data = json!({
        "value": 50,
        "multiplier": 2
    });

    let args = RsJsBridgeArgs {
        func_name: "process_data".to_string(),
        args: vec![("data".to_string(), input_data)].into_iter().collect(),
    };
    let args_json = args.to_string().unwrap();

    let result = rsjs_bridge_core(&mut op_state, &args_json, "com.sapphillon.test.math-plugin");
    assert!(
        result.is_ok(),
        "Bridge execution failed: {:?}",
        result.err()
    );

    let result_json = result.unwrap();
    let returns = RsJsBridgeReturns::new_from_str(&result_json).expect("Failed to parse returns");

    let result_obj = returns.args.get("result").expect("No result returned");

    assert_eq!(result_obj.get("original"), Some(&json!(50)));
    assert_eq!(result_obj.get("result"), Some(&json!(100)));
    assert!(result_obj.get("timestamp").is_some());
}

/// Integration Test: Plugin Throws Error
///
/// **Note:** This test requires the external plugin server binary.
#[test]
#[ignore = "Requires external plugin server binary"]
fn test_bridge_error_handling() {
    use std::collections::HashMap;

    let (mut op_state, _tokio_rt) =
        create_opstate_with_package(ERROR_PLUGIN_JS, "error-plugin", "com.sapphillon.test");

    let args_immediate = RsJsBridgeArgs {
        func_name: "throw_immediate".to_string(),
        args: HashMap::new(),
    };
    let result_immediate = rsjs_bridge_core(
        &mut op_state,
        &args_immediate.to_string().unwrap(),
        "com.sapphillon.test.error-plugin",
    );

    assert!(
        result_immediate.is_err(),
        "Expected error from throw_immediate, got Ok"
    );
    let err_msg = result_immediate.err().unwrap().to_string();
    assert!(
        err_msg.contains("This is an immediate error"),
        "Expected error message to contain 'This is an immediate error', got: {err_msg}"
    );
}

/// Integration Test: Unknown Function Call
///
/// **Note:** This test requires the external plugin server binary.
#[test]
#[ignore = "Requires external plugin server binary"]
fn test_bridge_unknown_function() {
    use std::collections::HashMap;

    let (mut op_state, _tokio_rt) =
        create_opstate_with_package(MATH_PLUGIN_JS, "math-plugin", "com.sapphillon.test");

    let args = RsJsBridgeArgs {
        func_name: "non_existent_func".to_string(),
        args: HashMap::new(),
    };

    let result = rsjs_bridge_core(
        &mut op_state,
        &args.to_string().unwrap(),
        "com.sapphillon.test.math-plugin",
    );

    assert!(
        result.is_err(),
        "Expected error for unknown function, got Ok"
    );
    let err_msg = result.err().unwrap().to_string();
    assert!(
        err_msg.contains("Unknown function") || err_msg.contains("schema not found"),
        "Expected 'Unknown function' error, got: {err_msg}"
    );
}

/// Integration Test: Loose Type Handling
///
/// **Note:** This test requires the external plugin server binary.
#[test]
#[ignore = "Requires external plugin server binary"]
fn test_bridge_loose_type_handling() {
    let (mut op_state, _tokio_rt) =
        create_opstate_with_package(MATH_PLUGIN_JS, "math-plugin", "com.sapphillon.test");

    // Pass strings instead of numbers
    let args = RsJsBridgeArgs {
        func_name: "add".to_string(),
        args: vec![
            ("a".to_string(), json!("10")),
            ("b".to_string(), json!("20")),
        ]
        .into_iter()
        .collect(),
    };

    let result = rsjs_bridge_core(
        &mut op_state,
        &args.to_string().unwrap(),
        "com.sapphillon.test.math-plugin",
    );

    assert!(
        result.is_ok(),
        "Bridge execution should succeed (JS is loose typed): {:?}",
        result.err()
    );

    let result_json = result.unwrap();
    let returns = RsJsBridgeReturns::new_from_str(&result_json).expect("Failed to parse returns");

    // JS `+` operator with strings does concatenation
    assert_eq!(
        returns.args.get("result"),
        Some(&json!("1020")),
        "Expected string concatenation result '1020'"
    );
}

/// Integration Test: Async Function Success
///
/// **Note:** This test requires the external plugin server binary.
#[test]
#[ignore = "Requires external plugin server binary"]
fn test_bridge_async_function_success() {
    let (mut op_state, _tokio_rt) =
        create_opstate_with_package(ERROR_PLUGIN_JS, "error-plugin", "com.sapphillon.test");

    let args = RsJsBridgeArgs {
        func_name: "async_success".to_string(),
        args: vec![("value".to_string(), json!("test-value"))]
            .into_iter()
            .collect(),
    };

    let result = rsjs_bridge_core(
        &mut op_state,
        &args.to_string().unwrap(),
        "com.sapphillon.test.error-plugin",
    );

    assert!(
        result.is_ok(),
        "Async function should succeed: {:?}",
        result.err()
    );

    let result_json = result.unwrap();
    let returns = RsJsBridgeReturns::new_from_str(&result_json).expect("Failed to parse returns");

    assert_eq!(
        returns.args.get("result"),
        Some(&json!("async: test-value")),
        "Expected async transformed result"
    );
}

// =============================================================================
// Integration Tests: Workflow Execution with External Plugins
// These tests require the external plugin server to be running.
// =============================================================================

/// Integration Test: External Plugin Execution via CoreWorkflowCode
///
/// **Purpose:**
/// Verify that `CoreWorkflowCode` can correctly execute external plugin functions.
///
/// **Note:** This test requires the external plugin server binary.
#[test]
#[ignore = "Requires external plugin server binary"]
fn test_workflow_with_external_plugin_add() {
    use sapphillon_core::plugin::CorePluginExternalFunction;

    let add_func = CorePluginExternalFunction::new(
        "math-plugin-add".to_string(),
        "add".to_string(),
        "Adds two numbers".to_string(),
        "mathPlugin".to_string(),
        MATH_PLUGIN_JS.to_string(),
        "com.sapphillon.test".to_string(),
    );

    let ext_package = CorePluginExternalPackage::new(
        "com.sapphillon.test.mathPlugin".to_string(),
        "mathPlugin".to_string(),
        vec![add_func],
        MATH_PLUGIN_JS.to_string(),
    );

    let workflow_code = r#"
        const result = com.sapphillon.test.mathPlugin.add(5, 7);
        console.log(result);
    "#;

    let mut code = CoreWorkflowCode::new(
        "test_wf_ext".to_string(),
        workflow_code.to_string(),
        vec![Arc::new(ext_package) as Arc<dyn PluginPackageTrait>],
        1,
        vec![],
        vec![],
    );

    let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
    code.run(tokio_runtime.handle().clone(), None, None);

    assert_eq!(code.result.len(), 1);
    let res = &code.result[0];
    assert_eq!(
        res.exit_code, 0,
        "Workflow should succeed, but got error: {}",
        res.result
    );
    assert!(
        res.result.contains("12"),
        "Expected result to contain '12', got: {}",
        res.result
    );
}

/// Integration Test: External Plugin Complex Object via CoreWorkflowCode
///
/// **Note:** This test requires the external plugin server binary.
#[test]
#[ignore = "Requires external plugin server binary"]
fn test_workflow_with_external_plugin_process_data() {
    use sapphillon_core::plugin::CorePluginExternalFunction;

    let process_func = CorePluginExternalFunction::new(
        "math-plugin-process-data".to_string(),
        "process_data".to_string(),
        "Processes data object".to_string(),
        "mathPlugin".to_string(),
        MATH_PLUGIN_JS.to_string(),
        "com.sapphillon.test".to_string(),
    );

    let ext_package = CorePluginExternalPackage::new(
        "com.sapphillon.test.mathPlugin".to_string(),
        "mathPlugin".to_string(),
        vec![process_func],
        MATH_PLUGIN_JS.to_string(),
    );

    let workflow_code = r#"
        const data = { value: 25, multiplier: 4 };
        const result = com.sapphillon.test.mathPlugin.process_data(data);
        console.log("Original:", result.original);
        console.log("Result:", result.result);
    "#;

    let mut code = CoreWorkflowCode::new(
        "test_wf_ext_complex".to_string(),
        workflow_code.to_string(),
        vec![Arc::new(ext_package) as Arc<dyn PluginPackageTrait>],
        1,
        vec![],
        vec![],
    );

    let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
    code.run(tokio_runtime.handle().clone(), None, None);

    assert_eq!(code.result.len(), 1);
    let res = &code.result[0];
    assert_eq!(
        res.exit_code, 0,
        "Workflow should succeed, but got error: {}",
        res.result
    );
    assert!(
        res.result.contains("Original: 25"),
        "Expected result to contain 'Original: 25', got: {}",
        res.result
    );
    assert!(
        res.result.contains("Result: 100"),
        "Expected result to contain 'Result: 100', got: {}",
        res.result
    );
}

/// Integration Test: Workflow Without Permission Requirement
///
/// **Note:** This test requires the external plugin server binary.
#[test]
#[ignore = "Requires external plugin server binary"]
fn test_workflow_without_permission_requirement() {
    use sapphillon_core::plugin::CorePluginExternalFunction;

    let simple_func = CorePluginExternalFunction::new(
        "file-plugin-simple-function".to_string(),
        "simple_function".to_string(),
        "A simple function".to_string(),
        "filePlugin".to_string(),
        FILE_PLUGIN_JS.to_string(),
        "com.sapphillon.test".to_string(),
    );

    let ext_package = CorePluginExternalPackage::new(
        "com.sapphillon.test.filePlugin".to_string(),
        "filePlugin".to_string(),
        vec![simple_func],
        FILE_PLUGIN_JS.to_string(),
    );

    let workflow_code = r#"
        const result = com.sapphillon.test.filePlugin.simple_function("Hello World");
        console.log("Result:", result);
    "#;

    let mut code = CoreWorkflowCode::new(
        "test_wf_no_permission_needed".to_string(),
        workflow_code.to_string(),
        vec![Arc::new(ext_package) as Arc<dyn PluginPackageTrait>],
        1,
        vec![],
        vec![],
    );

    let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
    code.run(tokio_runtime.handle().clone(), None, None);

    assert_eq!(code.result.len(), 1);
    let res = &code.result[0];
    assert_eq!(
        res.exit_code, 0,
        "Workflow should succeed, but got error: {}",
        res.result
    );
    assert!(
        res.result.contains("Echo: Hello World"),
        "Expected echoed text, got: {}",
        res.result
    );
}

// =============================================================================
// Integration Tests: End-to-End Installation and Execution Flow
// =============================================================================

/// Integration Test: Complete Install → Load → Execute Flow
///
/// **Purpose:**
/// Verify the complete end-to-end flow of:
/// 1. Installing an external plugin to the filesystem
/// 2. Loading the plugin from the installed location
/// 3. Executing a workflow that uses the installed plugin
///
/// **Note:** This test requires the external plugin server binary.
#[test]
#[ignore = "Requires external plugin server binary"]
fn test_complete_install_load_execute_flow() {
    use sapphillon_core::plugin::CorePluginExternalFunction;

    // Step 1: Simulate plugin installation
    let author_id = "com.sapphillon.test";
    let package_name = "installed-plugin";
    let version = "1.0.0";
    
    let (_temp_dir, package_js_path) =
        create_temp_plugin(author_id, package_name, version, MATH_PLUGIN_JS);

    // Step 2: Verify installation
    assert!(
        package_js_path.exists(),
        "Plugin should be installed at {:?}",
        package_js_path
    );

    // Step 3: Load plugin from installed location
    let loaded_js = std::fs::read_to_string(&package_js_path)
        .expect("Failed to read installed plugin");

    // Step 4: Create plugin package from loaded code
    let add_func = CorePluginExternalFunction::new(
        "installed-plugin-add".to_string(),
        "add".to_string(),
        "Adds two numbers".to_string(),
        "installedPlugin".to_string(),
        loaded_js.clone(),
        author_id.to_string(),
    );

    let ext_package = CorePluginExternalPackage::new(
        format!("{author_id}.installedPlugin"),
        "installedPlugin".to_string(),
        vec![add_func],
        loaded_js,
    );

    // Step 5: Execute workflow with loaded plugin
    let workflow_code = format!(
        r#"
        const result = {author_id}.installedPlugin.add(100, 200);
        console.log("Installed plugin result:", result);
    "#
    );

    let mut code = CoreWorkflowCode::new(
        "test_install_load_execute".to_string(),
        workflow_code,
        vec![Arc::new(ext_package) as Arc<dyn PluginPackageTrait>],
        1,
        vec![],
        vec![],
    );

    let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
    code.run(tokio_runtime.handle().clone(), None, None);

    // Step 6: Verify results
    assert_eq!(code.result.len(), 1);
    let res = &code.result[0];
    assert_eq!(
        res.exit_code, 0,
        "Workflow should succeed, but got error: {}",
        res.result
    );
    assert!(
        res.result.contains("300"),
        "Expected '300' (100+200), got: {}",
        res.result
    );
}

/// Integration Test: Multiple Plugins in Single Workflow
///
/// **Note:** This test requires the external plugin server binary.
#[test]
#[ignore = "Requires external plugin server binary"]
fn test_multiple_plugins_in_workflow() {
    use sapphillon_core::plugin::CorePluginExternalFunction;

    let add_func = CorePluginExternalFunction::new(
        "math-add".to_string(),
        "add".to_string(),
        "Adds two numbers".to_string(),
        "mathPlugin".to_string(),
        MATH_PLUGIN_JS.to_string(),
        "com.sapphillon.test".to_string(),
    );

    let math_package = CorePluginExternalPackage::new(
        "com.sapphillon.test.mathPlugin".to_string(),
        "mathPlugin".to_string(),
        vec![add_func],
        MATH_PLUGIN_JS.to_string(),
    );

    let simple_func = CorePluginExternalFunction::new(
        "file-simple".to_string(),
        "simple_function".to_string(),
        "Echoes a message".to_string(),
        "filePlugin".to_string(),
        FILE_PLUGIN_JS.to_string(),
        "com.sapphillon.test".to_string(),
    );

    let file_package = CorePluginExternalPackage::new(
        "com.sapphillon.test.filePlugin".to_string(),
        "filePlugin".to_string(),
        vec![simple_func],
        FILE_PLUGIN_JS.to_string(),
    );

    let workflow_code = r#"
        const sum = com.sapphillon.test.mathPlugin.add(10, 20);
        const message = com.sapphillon.test.filePlugin.simple_function("Sum is: " + sum);
        console.log(message);
    "#;

    let mut code = CoreWorkflowCode::new(
        "test_multi_plugin".to_string(),
        workflow_code.to_string(),
        vec![
            Arc::new(math_package) as Arc<dyn PluginPackageTrait>,
            Arc::new(file_package) as Arc<dyn PluginPackageTrait>,
        ],
        1,
        vec![],
        vec![],
    );

    let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
    code.run(tokio_runtime.handle().clone(), None, None);

    assert_eq!(code.result.len(), 1);
    let res = &code.result[0];
    assert_eq!(
        res.exit_code, 0,
        "Multi-plugin workflow should succeed, but got error: {}",
        res.result
    );
    assert!(
        res.result.contains("Echo: Sum is: 30"),
        "Expected echoed sum message, got: {}",
        res.result
    );
}

/// Integration Test: Plugin Reinstallation (Overwrite)
///
/// **Purpose:**
/// Verify that a plugin can be reinstalled (overwritten) with new code.
///
/// **Note:** This test requires the external plugin server binary.
#[test]
#[ignore = "Requires external plugin server binary"]
fn test_plugin_reinstallation_workflow() {
    use sapphillon_core::plugin::CorePluginExternalFunction;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let plugin_dir = temp_dir.path().join("author/plugin/1.0.0");
    std::fs::create_dir_all(&plugin_dir).expect("Failed to create dir");

    // Initial installation - multiplies by 2
    let initial_plugin = r#"
globalThis.Sapphillon = {
    Package: {
        meta: { 
            name: "calc", 
            version: "1.0.0", 
            author_id: "test", 
            package_id: "test.calc",
            description: "Calculator plugin"
        },
        functions: {
            calculate: {
                description: "Calculates value",
                permissions: [],
                parameters: [{ idx: 0, name: "x", type: "number", description: "Input number" }],
                returns: [{ idx: 0, type: "number", description: "Result" }],
                handler: (x) => x * 2
            }
        }
    }
};
"#;

    let package_path = plugin_dir.join("package.js");
    std::fs::write(&package_path, initial_plugin).expect("Failed to write initial plugin");

    // Load and execute initial version
    let loaded_v1 = std::fs::read_to_string(&package_path).expect("Failed to read v1");
    
    let calc_func_v1 = CorePluginExternalFunction::new(
        "calc".to_string(),
        "calculate".to_string(),
        "Calculates".to_string(),
        "calcPlugin".to_string(),
        loaded_v1.clone(),
        "test".to_string(),
    );

    let package_v1 = CorePluginExternalPackage::new(
        "test.calcPlugin".to_string(),
        "calcPlugin".to_string(),
        vec![calc_func_v1],
        loaded_v1,
    );

    let workflow_v1 = r#"
        console.log(test.calcPlugin.calculate(10));
    "#;

    let mut code_v1 = CoreWorkflowCode::new(
        "test_v1".to_string(),
        workflow_v1.to_string(),
        vec![Arc::new(package_v1) as Arc<dyn PluginPackageTrait>],
        1,
        vec![],
        vec![],
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    code_v1.run(rt.handle().clone(), None, None);

    assert!(
        code_v1.result[0].result.contains("20"),
        "V1 should return 20 (10*2), got: {}",
        code_v1.result[0].result
    );

    // Reinstall with new version - multiplies by 3
    let updated_plugin = r#"
globalThis.Sapphillon = {
    Package: {
        meta: { 
            name: "calc", 
            version: "1.0.0", 
            author_id: "test", 
            package_id: "test.calc",
            description: "Calculator plugin updated"
        },
        functions: {
            calculate: {
                description: "Calculates value",
                permissions: [],
                parameters: [{ idx: 0, name: "x", type: "number", description: "Input number" }],
                returns: [{ idx: 0, type: "number", description: "Result" }],
                handler: (x) => x * 3
            }
        }
    }
};
"#;

    std::fs::write(&package_path, updated_plugin).expect("Failed to write updated plugin");

    // Load and execute updated version
    let loaded_v2 = std::fs::read_to_string(&package_path).expect("Failed to read v2");
    
    let calc_func_v2 = CorePluginExternalFunction::new(
        "calc".to_string(),
        "calculate".to_string(),
        "Calculates".to_string(),
        "calcPlugin".to_string(),
        loaded_v2.clone(),
        "test".to_string(),
    );

    let package_v2 = CorePluginExternalPackage::new(
        "test.calcPlugin".to_string(),
        "calcPlugin".to_string(),
        vec![calc_func_v2],
        loaded_v2,
    );

    let workflow_v2 = r#"
        console.log(test.calcPlugin.calculate(10));
    "#;

    let mut code_v2 = CoreWorkflowCode::new(
        "test_v2".to_string(),
        workflow_v2.to_string(),
        vec![Arc::new(package_v2) as Arc<dyn PluginPackageTrait>],
        1,
        vec![],
        vec![],
    );

    let rt2 = tokio::runtime::Runtime::new().unwrap();
    code_v2.run(rt2.handle().clone(), None, None);

    assert!(
        code_v2.result[0].result.contains("30"),
        "V2 should return 30 (10*3), got: {}",
        code_v2.result[0].result
    );
}
