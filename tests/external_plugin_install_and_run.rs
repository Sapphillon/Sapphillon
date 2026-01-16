// Sapphillon
// SPDX-FileCopyrightText: 2025 Yuta Takahashi
// SPDX-License-Identifier: MPL-2.0 OR GPL-3.0-or-later

use ext_plugin::{RsJsBridgeArgs, SapphillonPackage};
use migration::MigratorTrait;
use sea_orm::{Database, DatabaseConnection};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[path = "../src/ext_plugin_manager.rs"]
mod ext_plugin_manager;
#[path = "../src/plugin_installer.rs"]
mod plugin_installer;

async fn setup_db() -> Result<DatabaseConnection, sea_orm::DbErr> {
    let db = Database::connect("sqlite::memory:").await?;
    migration::Migrator::up(&db, None).await?;
    Ok(db)
}

#[tokio::test]
async fn installs_and_runs_external_plugin() -> anyhow::Result<()> {
    let db = setup_db().await?;
    let save_dir = TempDir::new()?;
    let source_dir = TempDir::new()?;

    let plugin_source_dir = source_dir
        .path()
        .join("app.sapphillon/javascript-example/1.0.0");
    fs::create_dir_all(&plugin_source_dir)?;

    let plugin_file = plugin_source_dir.join("package.js");
    let package_js = r#"
/**
 * 2つの数値を加算します。
 * @param {number} a - 足される数
 * @param {number} b - 足す数
 * @returns {number} 合計
 * @permission [\"FileSystemRead:/etc\", \"FileSystemWrite:/etc\"]
 */
function add(a, b) {
  return a + b;
}

Sapphillon.Package = {
  meta: {
    name: "JavaScript Example",
    version: "1.0.0",
    description: "A simple JavaScript-based plugin example.",
    author_id: "app.sapphillon",
    package_id: "app.sapphillon.javascript-example"
  },
  functions: {
    add: {
      handler: add,
      permissions: [{type: "FileSystemRead", resource: "/etc"}, {type: "FileSystemWrite", resource: "/etc"}],
      description: "2つの数値を加算します。",
      parameters: [
        { name: "a", idx: 0, type: "number", description: "足される数" },
        { name: "b", idx: 1, type: "number", description: "足す数" }
      ],
      returns: [
        { type: "number", idx: 0, description: "合計" }
      ]
    }
  }
};
"#;
    fs::write(&plugin_file, package_js)?;

    let file_uri = format!("file://{}", plugin_file.to_string_lossy());
    let install_result = plugin_installer::install_plugin_from_uri(
        &db,
        &save_dir.path().to_string_lossy(),
        &file_uri,
    )
    .await?;

    let installed_path = save_dir
        .path()
        .join("app.sapphillon/javascript-example/1.0.0/package.js");
    assert!(installed_path.exists());
    assert_eq!(
        install_result.plugin_package_id,
        "app.sapphillon/javascript-example/1.0.0"
    );

    let installed_js = fs::read_to_string(installed_path)?;
    let package = SapphillonPackage::new(&installed_js)?;

    let args = RsJsBridgeArgs {
        func_name: "add".to_string(),
        args: vec![("a".to_string(), json!(1)), ("b".to_string(), json!(5))]
            .into_iter()
            .collect(),
    };
    let returns = package.execute(args, &None).await?;

    let result_value = returns
        .args
        .get("result")
        .and_then(|value| value.as_i64())
        .unwrap_or_default();
    assert_eq!(result_value, 6);

    Ok(())
}
