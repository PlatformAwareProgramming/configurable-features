
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::{env, fs};
use std::error::Error;

use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use std::collections::HashMap;


pub static CURRENT_FEATURES: Lazy<Mutex<PlatformFeatures>> = Lazy::new(|| {
    let m = HashMap::new(); // readplatformfeatures();
    Mutex::new(m)
});

static CURRENT_CONFIG_STRING: Lazy<Option<String>> = Lazy::new(|| {
    readplatformfilecontents().ok()
});

fn readplatformfilecontents() -> Result<String,Box<dyn Error>> {

    let platform_path = match env::var("PLATFORM_DESCRIPTION") {
        Ok(var) => var,
        Err(_) => env::var("PWD")?
    };

    let contents: String = fs::read_to_string(Path::new(&platform_path).join("Platform.toml"))?;

    Ok(contents)
}


pub fn readplatform<P:DeserializeOwned>() -> Result<P,Box<dyn Error>> {

    let contents = CURRENT_CONFIG_STRING.clone().expect("error reading Platform.toml");
    
    Ok(toml::from_str(&contents).unwrap())
}



use crate::{lookup_feature, PlatformFeatures, PlatformParameter};

use super::Feature;


pub fn add_qualifier(m: &mut HashMap<PlatformParameter, Arc<dyn Feature + Send + Sync>>, par:PlatformParameter, v:String) { 
    let f = lookup_feature(Box::leak(v.into_boxed_str()));
    match f {
        Some(f) => { m.insert(par, f); },
        None => {},
    }
}

pub fn add_quantifier(m: &mut HashMap<PlatformParameter, Arc<dyn Feature + Send + Sync>>, par:PlatformParameter, v:i32) { 
    m.insert(par, Arc::new(v));
}