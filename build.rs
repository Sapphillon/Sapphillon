// Sapphillon
// SPDX-FileCopyrightText: 2025 Yuta Takahashi
// SPDX-License-Identifier: MPL-2.0 OR GPL-3.0-or-later

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO Re-enable Windows support
    #[cfg(target_os = "windows")]
    compile_error!("Currently, Windows support is suspended.");

    // Discover and register internal plugins from js_plugins directory
    discover_internal_plugins()?;

    Ok(())
}

/// Represents information about a discovered internal plugin
struct PluginInfo {
    plugin_package_id: String,
    install_dir: PathBuf,
    package_name: String,
    package_version: String,
}

/// Discovers internal plugins from js_plugins directory and generates Rust code
fn discover_internal_plugins() -> Result<(), Box<dyn std::error::Error>> {
    let js_plugins_dir = Path::new("js_plugins");

    // Check if js_plugins directory exists
    if !js_plugins_dir.exists() {
        println!(
            "cargo:warning=js_plugins directory not found, skipping internal plugin discovery"
        );
        return Ok(());
    }

    // Use BTreeMap for stable alphabetical ordering
    let mut plugins: BTreeMap<String, PluginInfo> = BTreeMap::new();

    // Recursively search for package.js files matching the pattern
    // Pattern: {author_id}/{package_id}/{version}/package.js
    if let Err(e) = find_package_files(js_plugins_dir, &mut plugins) {
        println!("cargo:warning=Error searching for package.js files: {e}");
        return Ok(());
    }

    if plugins.is_empty() {
        println!("cargo:warning=No package.js files found in js_plugins directory");
        return Ok(());
    }

    // Generate the internal_plugins.rs file
    generate_internal_plugins_file(&plugins)?;

    // Print summary of discovered plugins
    println!(
        "cargo:warning=Discovered {} internal plugin(s):",
        plugins.len()
    );
    for plugin in plugins.values() {
        println!(
            "cargo:warning=  - {} (name: {}, version: {})",
            plugin.plugin_package_id, plugin.package_name, plugin.package_version
        );
    }

    Ok(())
}

/// Recursively searches for package.js files matching the pattern
/// {author_id}/{package_id}/{version}/package.js
fn find_package_files(
    dir: &Path,
    plugins: &mut BTreeMap<String, PluginInfo>,
) -> Result<(), Box<dyn std::error::Error>> {
    let entries = fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively search subdirectories
            find_package_files(&path, plugins)?;
        } else if path.file_name().is_some_and(|name| name == "package.js") {
            // Found a package.js file, extract plugin information
            if let Some(plugin_info) = extract_plugin_info(&path) {
                plugins.insert(plugin_info.plugin_package_id.clone(), plugin_info);
            }
        }
    }

    Ok(())
}

/// Extracts plugin information from a package.js file path
/// Expected path pattern: js_plugins/{author_id}/{package_id}/{version}/package.js
fn extract_plugin_info(package_path: &Path) -> Option<PluginInfo> {
    // Get the path components relative to js_plugins directory
    let components: Vec<&str> = package_path
        .iter()
        .skip_while(|&c| c != "js_plugins")
        .skip(1) // Skip "js_plugins"
        .map(|c| c.to_str())
        .collect::<Option<Vec<_>>>()?;

    // Expected pattern: {author_id}/{package_id}/{version}/package.js
    if components.len() != 4 || components[3] != "package.js" {
        println!(
            "cargo:warning=Skipping package.js with unexpected path structure: {package_path:?}"
        );
        return None;
    }

    let author_id = components[0];
    let package_id = components[1];
    let version = components[2];

    // Validate version format (basic semantic versioning check)
    if !is_valid_version(version) {
        println!(
            "cargo:warning=Skipping plugin with invalid version '{version}': {package_path:?}"
        );
        return None;
    }

    // Build plugin_package_id in format: {author_id}/{package_id}/{version}
    let plugin_package_id = format!("{author_id}/{package_id}/{version}");

    // Get the install directory (the directory containing package.js)
    let install_dir = package_path.parent()?.to_path_buf();

    Some(PluginInfo {
        plugin_package_id,
        install_dir,
        package_name: package_id.to_string(),
        package_version: version.to_string(),
    })
}

/// Validates a version string (basic semantic versioning check)
fn is_valid_version(version: &str) -> bool {
    // Basic check: version should start with a digit
    version.chars().next().is_some_and(|c| c.is_ascii_digit())
}

/// Generates the src/internal_plugins.rs file with discovered plugin information
fn generate_internal_plugins_file(
    plugins: &BTreeMap<String, PluginInfo>,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = Path::new("src/internal_plugins.rs");

    // Build the generated code
    let mut code = String::new();

    // File header
    code.push_str("// Sapphillon\n");
    code.push_str("// SPDX-FileCopyrightText: 2025 Yuta Takahashi\n");
    code.push_str("// SPDX-License-Identifier: MPL-2.0 OR GPL-3.0-or-later\n");
    code.push_str("//\n");
    code.push_str("// This file is automatically generated by build.rs\n");
    code.push_str("// DO NOT EDIT THIS FILE MANUALLY\n");
    code.push_str("//\n");
    code.push_str("// This file contains information about internal plugins discovered\n");
    code.push_str("// from the js_plugins directory during build time.\n");
    code.push('\n');
    code.push_str("use entity::entity::plugin_package::Model as PluginPackage;\n");
    code.push('\n');
    code.push_str("/// Returns a list of internal plugins discovered at build time.\n");
    code.push_str("///\n");
    code.push_str("/// This function returns plugin information for all internal plugins\n");
    code.push_str("/// found in the js_plugins directory. The plugins are returned in\n");
    code.push_str("/// alphabetical order by package_id for stable ordering.\n");
    code.push_str("pub fn internal_plugins() -> Vec<PluginPackage> {\n");
    code.push_str("    let mut plugins = Vec::new();\n");
    code.push('\n');

    // Generate plugin entries
    for (plugin_package_id, plugin) in plugins {
        // Escape backslashes in install_dir path for Windows compatibility
        let _install_dir_escaped = plugin
            .install_dir
            .display()
            .to_string()
            .replace("\\", "\\\\");

        code.push_str("    plugins.push(PluginPackage {\n");
        code.push_str(&format!(
            "        package_id: \"{}\".to_string(),\n",
            escape_string(plugin_package_id)
        ));
        code.push_str(&format!(
            "        package_name: \"{}\".to_string(),\n",
            escape_string(&plugin.package_name)
        ));
        code.push_str(&format!(
            "        package_version: \"{}\".to_string(),\n",
            escape_string(&plugin.package_version)
        ));
        code.push_str("        description: None,\n");
        code.push_str("        plugin_store_url: None,\n");
        code.push_str("        internal_plugin: true,\n");
        code.push_str("        verified: true,\n");
        code.push_str("        deprecated: false,\n");
        code.push_str("        installed_at: None,\n");
        code.push_str("        updated_at: None,\n");
        code.push_str("    });\n");
        code.push('\n');
    }

    code.push_str("    plugins\n");
    code.push_str("}\n");

    // Write the generated code to the file
    let mut file = fs::File::create(output_path)?;
    file.write_all(code.as_bytes())?;

    // Tell cargo to re-run this build script if js_plugins directory changes
    println!("cargo:rerun-if-changed=js_plugins");

    Ok(())
}

/// Escapes special characters in strings for Rust code generation
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
