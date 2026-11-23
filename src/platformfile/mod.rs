
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex, Once};
use std::{env, fs};
use std::error::Error;

use serde::de::DeserializeOwned;
use std::collections::HashMap;


pub static CURRENT_FEATURES: LazyLock<Mutex<PlatformFeatures>> = LazyLock::new(|| {
    let m = HashMap::new();
    Mutex::new(m)
});

type FeatureLoader = Box<dyn Fn() -> PlatformFeatures + Send + Sync>;

static FEATURE_LOADERS: LazyLock<Mutex<Vec<FeatureLoader>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

static FEATURES_LOADED: Once = Once::new();

/// Registers a deferred loader that extracts features from `Platform.toml`.
///
/// Loaders are executed lazily on first access to `PlatformRegistry::current_features()`,
/// after all `#[ctor]` qualifier registrations across core, vendor, and third-party crates
/// have completed.
pub fn register_feature_loader<F>(loader: F)
where
    F: Fn() -> PlatformFeatures + Send + Sync + 'static,
{
    FEATURE_LOADERS.lock().unwrap().push(Box::new(loader));
}

/// Runs all registered feature loaders once and merges their results into `CURRENT_FEATURES`.
pub fn ensure_features_loaded() {
    FEATURES_LOADED.call_once(|| {
        let mut merged = PlatformFeatures::new();
        {
            let loaders = FEATURE_LOADERS.lock().unwrap();
            if loaders.is_empty() {
                return;
            }
            for loader in loaders.iter() {
                merged.extend(loader());
            }
        }
        CURRENT_FEATURES.lock().unwrap().extend(merged);
    });
}

/// Marks deferred feature loading as already satisfied (used when features are set explicitly).
pub(crate) fn mark_features_loaded() {
    FEATURES_LOADED.call_once(|| {});
}

static CURRENT_CONFIG_STRING: LazyLock<Option<String>> = LazyLock::new(|| {
    readplatformfilecontents().ok()
});

fn readplatformfilecontents() -> Result<String, Box<dyn Error>> {
    let platform_path = match env::var("PLATFORM_DESCRIPTION") {
        Ok(var) => PathBuf::from(var),
        Err(_) => match env::var("PWD") {
            Ok(pwd) => PathBuf::from(pwd),
            Err(_) => env::current_dir()?,
        },
    };

    let contents: String = fs::read_to_string(platform_path.join("Platform.toml"))?;

    Ok(contents)
}


pub fn readplatform<P: DeserializeOwned>() -> Result<P, Box<dyn Error>> {
    let contents = CURRENT_CONFIG_STRING
        .as_deref()
        .ok_or("error reading Platform.toml")?;

    Ok(toml::from_str(contents)?)
}



use crate::{lookup_feature, PlatformFeatures, PlatformParameter};

use super::Feature;


pub fn add_qualifier(m: &mut HashMap<PlatformParameter, Arc<dyn Feature>>, par: PlatformParameter, v: String) { 
    if let Some(f) = lookup_feature(&v) {
        m.insert(par, f);
    }
}

pub fn add_quantifier(m: &mut HashMap<PlatformParameter, Arc<dyn Feature>>, par: PlatformParameter, v: i32) { 
    m.insert(par, Arc::new(v));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_readplatform_returns_err_instead_of_panic_when_file_missing() {
        let res = readplatform::<toml::Value>();
        assert!(res.is_err());
    }
}