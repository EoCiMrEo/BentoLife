use std::{env, path::PathBuf};

use bentolife_core::domain::vault::VaultService;

pub fn default_vault_path() -> Result<PathBuf, String> {
    let env_key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    let home_path = env::var_os(env_key)
        .map(PathBuf::from)
        .ok_or_else(|| format!("Unable to resolve {env_key} for default vault location."))?;
    Ok(VaultService::default_vault_path_from_home(home_path))
}
