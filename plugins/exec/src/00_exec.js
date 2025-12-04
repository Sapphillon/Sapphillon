// Sapphillon
// SPDX-FileCopyrightText: 2025 Yuta Takahashi
// SPDX-License-Identifier: MPL-2.0 OR GPL-3.0-or-later

((globalThis) => {
    const core = Deno.core;

    function exec(command) {
        return core.ops.op2_exec(command);
    }

    globalThis.exec = exec;
})(globalThis);
