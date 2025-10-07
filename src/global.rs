// Sapphillon
// Copyright 2025 Yuta Takahashi
//
// This file is part of Sapphillon
//
// Sapphillon is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use tokio::sync::RwLock;
use std::sync::LazyLock;

#[derive(Debug)]
pub struct GlobalStateData {
    db_initialized: bool,
    db_url: String
}

#[derive(Debug)]
pub struct GlobalState {
    pub data: LazyLock<RwLock<GlobalStateData>>,
}

impl GlobalState {
    pub const fn new() -> Self {
        GlobalState {
            data: LazyLock::new(|| RwLock::new(GlobalStateData {
                db_initialized: false,
                db_url: String::new(),
            })),
        }
    }
}

impl std::fmt::Display for GlobalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Use the non-blocking try_read so we don't block the current thread
        // (blocking_read would panic if called from a Tokio runtime thread).
        match self.data.try_read() {
            Ok(data) => write!(f, "GlobalState {{ db_initialized: {}, db_url: '{}' }}", data.db_initialized, data.db_url),
            Err(_) => write!(f, "GlobalState {{ <locked> }}"),
        }
    }
}

