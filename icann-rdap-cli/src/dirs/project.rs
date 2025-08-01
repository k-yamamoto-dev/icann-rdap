use std::{
    fs::{create_dir_all, remove_dir_all, write},
    path::PathBuf,
    sync::LazyLock,
};

use directories::ProjectDirs;

pub const QUALIFIER: &str = "org";
pub const ORGANIZATION: &str = "ICANN";
pub const APPLICATION: &str = "rdap";

pub const ENV_FILE_NAME: &str = "rdap.env";
pub const RDAP_CACHE_NAME: &str = "rdap_cache";
pub const BOOTSTRAP_CACHE_NAME: &str = "bootstrap_cache";

pub(crate) static PROJECT_DIRS: LazyLock<ProjectDirs> = LazyLock::new(|| {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
        .expect("unable to formulate project directories")
});

/// Initializes the directories to be used.
pub fn init() -> Result<(), std::io::Error> {
    create_dir_all(PROJECT_DIRS.config_dir())?;
    create_dir_all(PROJECT_DIRS.cache_dir())?;
    create_dir_all(rdap_cache_path())?;
    create_dir_all(bootstrap_cache_path())?;

    // create default config file
    if !config_path().exists() {
        let example_config = include_str!("rdap.env");
        write(config_path(), example_config)?;
    }
    Ok(())
}

/// Reset the directories.
pub fn reset() -> Result<(), std::io::Error> {
    remove_dir_all(PROJECT_DIRS.config_dir())?;
    remove_dir_all(PROJECT_DIRS.cache_dir())?;
    init()
}

/// Returns a [PathBuf] to the configuration file.
pub fn config_path() -> PathBuf {
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config).join(ENV_FILE_NAME)
    } else {
        PROJECT_DIRS.config_dir().join(ENV_FILE_NAME)
    }
}

/// Returns a [PathBuf] to the cache directory for RDAP responses.
pub fn rdap_cache_path() -> PathBuf {
    PROJECT_DIRS.cache_dir().join(RDAP_CACHE_NAME)
}

/// Returns a [PathBuf] to the cache directory for bootstrap files.
pub fn bootstrap_cache_path() -> PathBuf {
    if let Ok(xdg_cache) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(xdg_cache).join(BOOTSTRAP_CACHE_NAME)
    } else {
        PROJECT_DIRS.cache_dir().join(BOOTSTRAP_CACHE_NAME)
    }
}
