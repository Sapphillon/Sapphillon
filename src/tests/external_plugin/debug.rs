// Sapphillon
// SPDX-FileCopyrightText: 2025 Yuta Takahashi
// SPDX-License-Identifier: MPL-2.0 OR GPL-3.0-or-later

//! Test with manually modified call script to test array argument hypothesis

use super::common::*;
use sapphillon_core::plugin::{CorePluginExternalPackage, PluginPackageTrait};
use sapphillon_core::workflow::CoreWorkflowCode;
use std::sync::Arc;

#[test]
#[ignore = "Testing array argument hypothesis"]
fn test_manual_call_script() {
    let file_plugin = read_fixture("file_plugin.js");

    // Create a package WITHOUT using CorePluginExternalFunction
    // We'll manually create the pre_script
    let manual_pre_script = r#"
(function() {
    const packageId = "com.sapphillon.test.filePlugin";
    const parts = packageId.split(".");
    let current = globalThis;
    for (const part of parts) {
        if (
            !Object.prototype.hasOwnProperty.call(current, part) ||
            typeof current[part] !== "object" ||
            current[part] === null
        ) {
            current[part] = {};
        }
        current = current[part];
    }
    current["simple_function"] = function(...args) {
        const bridgeArgs = {
            func_name: "simple_function",
            args: {}
        };
        args.forEach((arg, idx) => {
            bridgeArgs.args[`arg${idx}`] = arg;
        });
        const argsJson = JSON.stringify(bridgeArgs);
        
        console.log("=== DEBUG: Before calling rsjs_bridge_opdecl ===");
        console.log("Type of argsJson:", typeof argsJson);
        console.log("argsJson:", argsJson);
        console.log("Type of packageId:", typeof packageId);
        console.log("packageId:", packageId);
        
        // Try calling with array
        try {
            console.log("Trying with array...");
            const resultJson = Deno.core.ops.rsjs_bridge_opdecl([argsJson, packageId]);
            console.log("Success with array!");
            const result = JSON.parse(resultJson);
            return result.args.result !== undefined ? result.args.result : result.args;
        } catch (e) {
            console.log("Failed with array:", e.toString());
            throw e;
        }
    };
})();
"#;

    // Create a custom package that includes the manual pre_script
    let ext_package = CorePluginExternalPackage::new(
        "com.sapphillon.test.filePlugin".to_string(),
        "filePlugin".to_string(),
        vec![], // No functions, we'll use manual pre_script
        file_plugin,
    );

    let workflow_code = format!(
        r#"
        {}
        
        const result = com.sapphillon.test.filePlugin.simple_function("Hello World");
        console.log("Result:", result);
    "#,
        manual_pre_script
    );

    let mut code = CoreWorkflowCode::new(
        "test_manual_call".to_string(),
        workflow_code,
        vec![Arc::new(ext_package) as Arc<dyn PluginPackageTrait>],
        1,
        vec![],
        vec![],
    );

    let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
    code.run(tokio_runtime.handle().clone(), None, None);

    println!("=== Workflow Results ===");
    for (i, res) in code.result.iter().enumerate() {
        println!("Result {}: exit_code={}", i, res.exit_code);
        println!("Output: {}", res.result);
    }
}
