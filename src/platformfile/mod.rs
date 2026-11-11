
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use std::{env, fs};
use std::error::Error;

use serde::de::DeserializeOwned;
use std::collections::HashMap;


pub static CURRENT_FEATURES: LazyLock<Mutex<PlatformFeatures>> = LazyLock::new(|| {
    let m = HashMap::new();
    Mutex::new(m)
});

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