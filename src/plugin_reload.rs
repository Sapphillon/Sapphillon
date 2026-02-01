// Sapphillon
// SPDX-FileCopyrightText: 2025 Yuta Takahashi
// SPDX-License-Identifier: MPL-2.0 OR GPL-3.0-or-later

//! Plugin hot-reload feature for development.
//!
//! When --reload flag is enabled, periodically scans `js_plugins` directory
//! for package.js files and registers/updates plugins in the database.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Result;
use log::{debug, info, warn};
use tokio::time::{Duration, interval};

use crate::GLOBAL_STATE;

/// Directory name (relative path from execution directory)
const JS_PLUGINS_DIR: &str = "js_plugins";
/// Scan interval in seconds
const SCAN_INTERVAL_SECS: u64 = 5;

/// Represents information about a discovered plugin
#[derive(Debug, Clone)]
struct PluginInfo {
    plugin_package_id: String,
    install_dir: PathBuf,
    package_name: String,
    package_version: String,
    author_id: String,
    content: String,
    modified_time: SystemTime,
}

/// Tracks the state of discovered plugins
#[derive(Debug, Clone)]
struct PluginState {
    content_hash: String,
    modified_time: SystemTime,
}

fn is_valid_version(version: &str) -> bool {
    // Basic check: version should start with a digit
    version.chars().next().map_or(false, |c| c.is_ascii_digit())
}

/// Extracts plugin information from a package.js file path
/// Expected path pattern: js_plugins/{author_id}/{package_id}/{version}/package.js
fn extract_plugin_info(package_path: &Path) -> Option<PluginInfo> {
    let components: Vec<&str> = package_path
        .iter()
        .skip_while(|&c| c != "js_plugins")
        .skip(1) // Skip "js_plugins"
        .map(|c| c.to_str())
        .collect::<Option<Vec<_>>>()?;

    // Expected pattern: {author_id}/{package_id}/{version}/package.js
    if components.len() != 4 || components[3] != "package.js" {
        return None;
    }

    let author_id = components[0];
    let package_id = components[1];
    let version = components[2];

    // Validate version format (basic semantic versioning check)
    if !is_valid_version(version) {
        return None;
    }

    // Build plugin_package_id in format: {author_id}/{package_id}/{version}
    let plugin_package_id = format!("{}/{}/{}", author_id, package_id, version);

    // Get the install directory (the directory containing package.js)
    let install_dir = package_path.parent()?.to_path_buf();

    // Read the package.js content
    let content = fs::read_to_string(package_path).ok()?;

    // Get file modification time
    let metadata = fs::metadata(package_path).ok()?;
    let modified_time = metadata.modified().ok()?;

    Some(PluginInfo {
        plugin_package_id,
        install_dir,
        package_name: package_id.to_string(),
        package_version: version.to_string(),
        author_id: author_id.to_string(),
        content,
        modified_time,
    })
}

/// Recursively searches for package.js files matching the pattern
/// {author_id}/{package_id}/{version}/package.js
fn find_package_files(
    dir: &Path,
    plugins: &mut BTreeMap<String, PluginInfo>,
) -> Result<()> {
    let entries = fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively search subdirectories
            find_package_files(&path, plugins)?;
        } else if path.file_name().map_or(false, |name| name == "package.js") {
            // Found a package.js file, extract plugin information
            if let Some(plugin_info) = extract_plugin_info(&path) {
                plugins.insert(plugin_info.plugin_package_id.clone(), plugin_info);
            }
        }
    }

    Ok(())
}

/// Scans the js_plugins directory for package.js files.
pub fn scan_js_plugins() -> Result<Vec<PluginInfo>> {
    let js_plugins_dir = Path::new(JS_PLUGINS_DIR);

    if !js_plugins_dir.exists() {
        debug!("js_plugins directory does not exist: {}", JS_PLUGINS_DIR);
        return Ok(vec![]);
    }

    // Use BTreeMap for stable alphabetical ordering
    let mut plugins: BTreeMap<String, PluginInfo> = BTreeMap::new();

    // Recursively search for package.js files
    find_package_files(js_plugins_dir, &mut plugins)?;

    Ok(plugins.into_values().collect())
}

/// Calculates a hash of the content for change detection
fn calculate_hash(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Creates a PluginPackage proto from discovered plugin info
fn create_plugin_package_proto(plugin: &PluginInfo) -> sapphillon_core::proto::sapphillon::v1::PluginPackage {
    use sapphillon_core::proto::sapphillon::v1::PluginPackage;

    PluginPackage {
        package_id: plugin.plugin_package_id.clone(),
        package_name: plugin.package_name.clone(),
        provider_id: plugin.author_id.clone(),
        package_version: plugin.package_version.clone(),
        description: String::new(),
        functions: vec![], // Empty function list (loaded dynamically at runtime)
        plugin_store_url: "BUILTIN".to_string(),
        internal_plugin: Some(true),
        verified: Some(true),
        deprecated: Some(false),
        installed_at: None,
        updated_at: None,
    }
}

/// Registers or updates a plugin in the database
async fn reload_plugin(plugin: &PluginInfo) -> Result<()> {
    use database::plugin::init_register_plugins;

    let db = GLOBAL_STATE.get_db_connection().await?;

    // Build PluginPackage proto
    let plugin_proto = create_plugin_package_proto(plugin);

    init_register_plugins(&db, vec![plugin_proto]).await?;

    info!("Successfully registered/updated plugin: {}", plugin.plugin_package_id);
    Ok(())
}

/// Starts the periodic plugin reload scanner.
///
/// This function runs in a loop, scanning the js_plugins directory every
/// `SCAN_INTERVAL_SECS` seconds and registering any new or changed plugins to the database.
pub async fn start_plugin_reload_scanner() {
    info!(
        "Starting js_plugins hot-reload scanner (interval: {}s)",
        SCAN_INTERVAL_SECS
    );

    // Wait for database to be fully initialized
    tokio::time::sleep(Duration::from_secs(2)).await;

    let mut known_plugins: HashMap<String, PluginState> = HashMap::new();
    let mut scanner_interval = interval(Duration::from_secs(SCAN_INTERVAL_SECS));

    loop {
        scanner_interval.tick().await;

        debug!("Scanning js_plugins directory for changes...");

        match scan_js_plugins() {
            Ok(plugins) => {
                if plugins.is_empty() {
                    debug!("No js_plugins found");
                    continue;
                }

                for plugin in plugins {
                    let content_hash = calculate_hash(&plugin.content);

                    if let Some(known) = known_plugins.get(&plugin.plugin_package_id) {
                        // Check if changed
                        if known.content_hash != content_hash {
                            info!("Reloading modified plugin: {}", plugin.plugin_package_id);
                            if let Err(e) = reload_plugin(&plugin).await {
                                warn!(
                                    "Failed to reload plugin '{}': {}",
                                    plugin.plugin_package_id, e
                                );
                            } else {
                                known_plugins.insert(
                                    plugin.plugin_package_id.clone(),
                                    PluginState {
                                        content_hash,
                                        modified_time: plugin.modified_time,
                                    },
                                );
                            }
                        }
                    } else {
                        // New plugin
                        info!("Registering new plugin: {}", plugin.plugin_package_id);
                        if let Err(e) = reload_plugin(&plugin).await {
                            warn!(
                                "Failed to register plugin '{}': {}",
                                plugin.plugin_package_id, e
                            );
                        } else {
                            known_plugins.insert(
                                plugin.plugin_package_id.clone(),
                                PluginState {
                                    content_hash,
                                    modified_time: plugin.modified_time,
                                },
                            );
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to scan js_plugins directory: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_version() {
        assert!(is_valid_version("1.0.0"));
        assert!(is_valid_version("2.3.4"));
        assert!(is_valid_version("0.0.1"));
        assert!(!is_valid_version("v1.0.0"));
        assert!(!is_valid_version("beta"));
        assert!(!is_valid_version(""));
    }

    #[test]
    fn test_calculate_hash() {
        let hash1 = calculate_hash("content1");
        let hash2 = calculate_hash("content1");
        let hash3 = calculate_hash("content2");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_extract_plugin_info_valid() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("js_plugins/author/pkg/1.0.0");
        fs::create_dir_all(&plugin_dir).unwrap();

        let package_js = plugin_dir.join("package.js");
        fs::write(
            &package_js,
            "globalThis.Sapphillon = { Package: { meta: { name: 'pkg', version: '1.0.0', author_id: 'author', package_id: 'pkg' }, functions: {} } };",
        ).unwrap();

        let result = extract_plugin_info(&package_js);
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.plugin_package_id, "author/pkg/1.0.0");
        assert_eq!(info.package_name, "pkg");
        assert_eq!(info.package_version, "1.0.0");
        assert_eq!(info.author_id, "author");
    }

    #[test]
    fn test_extract_plugin_info_invalid_version() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("js_plugins/author/pkg/v1.0.0");
        fs::create_dir_all(&plugin_dir).unwrap();

        let package_js = plugin_dir.join("package.js");
        fs::write(&package_js, "content").unwrap();

        let result = extract_plugin_info(&package_js);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_plugin_info_wrong_path() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("wrong_path/author/pkg/1.0.0");
        fs::create_dir_all(&plugin_dir).unwrap();

        let package_js = plugin_dir.join("package.js");
        fs::write(&package_js, "content").unwrap();

        let result = extract_plugin_info(&package_js);
        assert!(result.is_none());
    }

    #[test]
    fn test_create_plugin_package_proto() {
        let plugin = PluginInfo {
            plugin_package_id: "author/pkg/1.0.0".to_string(),
            install_dir: PathBuf::from("/tmp/author/pkg/1.0.0"),
            package_name: "pkg".to_string(),
            package_version: "1.0.0".to_string(),
            author_id: "author".to_string(),
            content: "content".to_string(),
            modified_time: SystemTime::now(),
        };

        let proto = create_plugin_package_proto(&plugin);
        assert_eq!(proto.package_id, "author/pkg/1.0.0");
        assert_eq!(proto.package_name, "pkg");
        assert_eq!(proto.package_version, "1.0.0");
        assert_eq!(proto.provider_id, "author");
        assert!(proto.internal_plugin.unwrap());
        assert!(proto.verified.unwrap());
        assert!(!proto.deprecated.unwrap());
        assert_eq!(proto.plugin_store_url, "BUILTIN");
        assert!(proto.functions.is_empty());
    }

    #[test]
    fn test_scan_js_plugins_empty() {
        let result = scan_js_plugins();
        assert!(result.is_ok());
        // js_plugins directory may not exist in test environment
    }
}
