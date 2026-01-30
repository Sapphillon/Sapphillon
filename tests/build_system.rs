// Sapphillon
// SPDX-FileCopyrightText: 2025 Yuta Takahashi
// SPDX-License-Identifier: MPL-2.0 OR GPL-3.0-or-later

//! Integration tests for the build system automation.
//!
//! This test suite verifies the functionality of the build system's
//! automatic discovery and registration of internal plugins.

use Sapphillon_Controller::internal_plugins;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: internal_plugins() returns the correct number of plugins
    ///
    /// **Purpose:**
    /// Verify that the internal_plugins() function returns the expected
    /// number of plugins discovered at build time.
    #[test]
    fn test_internal_plugins_count() {
        let plugins = internal_plugins();
        
        // Verify that exactly 4 plugins are returned
        assert_eq!(
            plugins.len(),
            4,
            "internal_plugins() should return exactly 4 plugins"
        );
    }

    /// Test: internal_plugins() returns correct plugin information
    ///
    /// **Purpose:**
    /// Verify that each plugin returned by internal_plugins() contains
    /// the correct information (package_id, package_name, package_version, etc.).
    #[test]
    fn test_internal_plugins_info() {
        let plugins = internal_plugins();
        
        // Expected plugin data
        let expected_plugins = vec![
            (
                "test/example/1.0.0",
                "example",
                "1.0.0",
            ),
            (
                "test/plugin-a/1.0.0",
                "plugin-a",
                "1.0.0",
            ),
            (
                "test/plugin-b/1.0.0",
                "plugin-b",
                "1.0.0",
            ),
            (
                "test/plugin-c/1.0.0",
                "plugin-c",
                "1.0.0",
            ),
        ];
        
        // Verify each plugin's information
        for plugin in &plugins {
            // Find the expected plugin data
            let expected = expected_plugins
                .iter()
                .find(|(id, _, _)| id == &plugin.package_id)
                .expect(&format!(
                    "Unexpected plugin_id found: {}",
                    plugin.package_id
                ));
            
            // Verify package_id
            assert_eq!(
                plugin.package_id,
                expected.0,
                "package_id should match for plugin {}",
                plugin.package_id
            );
            
            // Verify package_name
            assert_eq!(
                plugin.package_name,
                expected.1,
                "package_name should match for plugin {}",
                plugin.package_id
            );
            
            // Verify package_version
            assert_eq!(
                plugin.package_version,
                expected.2,
                "package_version should match for plugin {}",
                plugin.package_id
            );
        }
    }

    /// Test: internal_plugins() sets internal_plugin flag to true
    ///
    /// **Purpose:**
    /// Verify that all plugins returned by internal_plugins() have
    /// the internal_plugin flag set to true.
    #[test]
    fn test_internal_plugins_flag() {
        let plugins = internal_plugins();
        
        // Verify that all plugins have internal_plugin set to true
        for plugin in &plugins {
            assert!(
                plugin.internal_plugin,
                "internal_plugin flag should be true for plugin {}",
                plugin.package_id
            );
        }
    }

    /// Test: internal_plugins() sets plugin_store_url to None
    ///
    /// **Purpose:**
    /// Verify that all plugins returned by internal_plugins() have
    /// the plugin_store_url set to None (indicating they are built-in).
    #[test]
    fn test_internal_plugins_store_url() {
        let plugins = internal_plugins();
        
        // Verify that all plugins have plugin_store_url set to None
        for plugin in &plugins {
            assert!(
                plugin.plugin_store_url.is_none(),
                "plugin_store_url should be None for plugin {}",
                plugin.package_id
            );
        }
    }

    /// Test: internal_plugins() returns plugins in alphabetical order
    ///
    /// **Purpose:**
    /// Verify that the plugins returned by internal_plugins() are
    /// sorted alphabetically by package_id for stable ordering.
    #[test]
    fn test_internal_plugins_order() {
        let plugins = internal_plugins();
        
        // Collect package_ids
        let package_ids: Vec<&str> = plugins.iter().map(|p| p.package_id.as_str()).collect();
        
        // Create a sorted version
        let mut sorted_ids = package_ids.clone();
        sorted_ids.sort();
        
        // Verify that the package_ids are sorted
        assert_eq!(
            package_ids, sorted_ids,
            "Plugins should be sorted alphabetically by package_id"
        );
    }

    /// Test: internal_plugins() sets verified flag to true
    ///
    /// **Purpose:**
    /// Verify that all plugins returned by internal_plugins() have
    /// the verified flag set to true.
    #[test]
    fn test_internal_plugins_verified_flag() {
        let plugins = internal_plugins();
        
        // Verify that all plugins have verified set to true
        for plugin in &plugins {
            assert!(
                plugin.verified,
                "verified flag should be true for plugin {}",
                plugin.package_id
            );
        }
    }

    /// Test: internal_plugins() sets deprecated flag to false
    ///
    /// **Purpose:**
    /// Verify that all plugins returned by internal_plugins() have
    /// the deprecated flag set to false.
    #[test]
    fn test_internal_plugins_deprecated_flag() {
        let plugins = internal_plugins();
        
        // Verify that all plugins have deprecated set to false
        for plugin in &plugins {
            assert!(
                !plugin.deprecated,
                "deprecated flag should be false for plugin {}",
                plugin.package_id
            );
        }
    }

    /// Test: internal_plugins() sets description to None
    ///
    /// **Purpose:**
    /// Verify that all plugins returned by internal_plugins() have
    /// the description set to None.
    #[test]
    fn test_internal_plugins_description() {
        let plugins = internal_plugins();
        
        // Verify that all plugins have description set to None
        for plugin in &plugins {
            assert!(
                plugin.description.is_none(),
                "description should be None for plugin {}",
                plugin.package_id
            );
        }
    }

    /// Test: internal_plugins() sets installed_at to None
    ///
    /// **Purpose:**
    /// Verify that all plugins returned by internal_plugins() have
    /// the installed_at timestamp set to None.
    #[test]
    fn test_internal_plugins_installed_at() {
        let plugins = internal_plugins();
        
        // Verify that all plugins have installed_at set to None
        for plugin in &plugins {
            assert!(
                plugin.installed_at.is_none(),
                "installed_at should be None for plugin {}",
                plugin.package_id
            );
        }
    }

    /// Test: internal_plugins() sets updated_at to None
    ///
    /// **Purpose:**
    /// Verify that all plugins returned by internal_plugins() have
    /// the updated_at timestamp set to None.
    #[test]
    fn test_internal_plugins_updated_at() {
        let plugins = internal_plugins();
        
        // Verify that all plugins have updated_at set to None
        for plugin in &plugins {
            assert!(
                plugin.updated_at.is_none(),
                "updated_at should be None for plugin {}",
                plugin.package_id
            );
        }
    }

    /// Test: internal_plugins() package_id format validation
    ///
    /// **Purpose:**
    /// Verify that all package_ids follow the expected format:
    /// {author_id}/{package_id}/{version}
    #[test]
    fn test_internal_plugins_package_id_format() {
        let plugins = internal_plugins();
        
        // Verify that all package_ids follow the expected format
        for plugin in &plugins {
            let parts: Vec<&str> = plugin.package_id.split('/').collect();
            
            assert_eq!(
                parts.len(),
                3,
                "package_id should have exactly 3 parts separated by '/' for plugin {}",
                plugin.package_id
            );
            
            // Verify that version starts with a digit (basic semantic versioning check)
            assert!(
                parts[2].chars().next().map_or(false, |c| c.is_ascii_digit()),
                "version should start with a digit for plugin {}",
                plugin.package_id
            );
        }
    }

    /// Test: internal_plugins() returns consistent results
    ///
    /// **Purpose:**
    /// Verify that calling internal_plugins() multiple times returns
    /// the same results (consistency check).
    #[test]
    fn test_internal_plugins_consistency() {
        let plugins1 = internal_plugins();
        let plugins2 = internal_plugins();
        
        // Verify that both calls return the same number of plugins
        assert_eq!(
            plugins1.len(),
            plugins2.len(),
            "internal_plugins() should return the same number of plugins on multiple calls"
        );
        
        // Verify that both calls return the same plugins
        for (p1, p2) in plugins1.iter().zip(plugins2.iter()) {
            assert_eq!(
                p1.package_id, p2.package_id,
                "package_id should be consistent across calls"
            );
            assert_eq!(
                p1.package_name, p2.package_name,
                "package_name should be consistent across calls"
            );
            assert_eq!(
                p1.package_version, p2.package_version,
                "package_version should be consistent across calls"
            );
            assert_eq!(
                p1.internal_plugin, p2.internal_plugin,
                "internal_plugin should be consistent across calls"
            );
        }
    }
}
