//! User accounts and logon state. Persisted as JSON under the platform data
//! directory. Modelled on the Windows XP multi-user / Welcome-screen model
//! (Reborn XP + Alena0490 references).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A single user account.
#[derive(Clone, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    /// Asset key of the user picture.
    pub avatar: String,
    /// FNV-1a hash of the password, hex encoded (empty = no password).
    pub password_hash: String,
    pub admin: bool,
}

impl Account {
    pub fn has_password(&self) -> bool {
        !self.password_hash.is_empty()
    }

    pub fn check(&self, password: &str) -> bool {
        self.password_hash.is_empty() || self.password_hash == hash(password)
    }
}

/// The account database.
#[derive(Clone, Serialize, Deserialize)]
pub struct Accounts {
    pub users: Vec<Account>,
}

impl Default for Accounts {
    fn default() -> Self {
        Self {
            users: vec![Account {
                name: "User".to_owned(),
                avatar: "login/avatar__skateboard.png".to_owned(),
                password_hash: String::new(),
                admin: true,
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
