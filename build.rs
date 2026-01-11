// Sapphillon
// SPDX-FileCopyrightText: 2025 Yuta Takahashi
// SPDX-License-Identifier: MPL-2.0 OR GPL-3.0-or-later

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO Resuport Windows
    #[cfg(target_os = "windows")]
    unimplemented!("Currently, Windows Support is suspended.");

    Ok(())
}
