//! User accounts + groups, persisted as JSON under the platform data directory.
//! Modelled on the ReactOS `usrmgr` control panel applet (the classic Windows XP
//! "User Accounts" manager — see `dll/cpl/usrmgr/lang/en-US.rc` for the exact
//! dialog text we mirror).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A single user account.
#[derive(Clone, Serialize, Deserialize)]
pub struct Account {
    /// Login name (the `Name` column).
    pub name: String,
    /// User picture asset key.
    pub avatar: String,
    /// FNV-1a hash of the password, hex encoded (empty = no password).
    pub password_hash: String,
    /// Member of the `Administrators` group (`Computer administrator`) rather
    /// than `Users` (`Limited`).
    pub admin: bool,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub disabled: bool,
}

impl Account {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            avatar: "users/chess.bmp".to_owned(),
            password_hash: String::new(),
            admin: false,
            full_name: String::new(),
            description: String::new(),
            disabled: false,
        }
    }

    pub fn has_password(&self) -> bool {
        !self.password_hash.is_empty()
    }

    pub fn check(&self, password: &str) -> bool {
        self.password_hash.is_empty() || self.password_hash == hash(password)
    }

    pub fn set_password(&mut self, password: &str) {
        self.password_hash = if password.is_empty() {
            String::new()
        } else {
            hash(password)
        };
    }
}

/// The account database, with the built-in groups.
#[derive(Clone, Serialize, Deserialize)]
pub struct Accounts {
    pub users: Vec<Account>,
}

impl Default for Accounts {
    fn default() -> Self {
        Self {
            users: vec![Account {
                name: "User".to_owned(),
                avatar: "users/chess.bmp".to_owned(),
                password_hash: String::new(),
                admin: true,
                full_name: String::new(),
                description: String::new(),
                disabled: false,
            }],
        }
    }
}

impl Accounts {
    pub fn load() -> Self {
        std::fs::read_to_string(path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(crate::settings::data_dir());
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path(), json);
        }
    }
}

/// FNV-1a hash of a password, hex encoded.
pub fn hash(password: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in password.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

fn path() -> PathBuf {
    crate::settings::data_dir().join("accounts.json")
}
