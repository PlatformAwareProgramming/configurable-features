use std::{collections::HashMap, sync::{Arc, LazyLock, Mutex}};

use super::Feature;

pub type PlatformParameter = String;


pub static PLATFORM_PARAMETERS: LazyLock<Mutex<Vec<PlatformParameter>>> = LazyLock::new(|| {
    let map: Vec<PlatformParameter> = Vec::new();
    Mutex::new(map)
});

pub type PlatformFeatures = HashMap<PlatformParameter, Arc<dyn Feature>>;


pub static FEATURE_TOP: LazyLock<Mutex<PlatformFeatures>> = LazyLock::new(|| {
    let map: PlatformFeatures = HashMap::new();    
    Mutex::new(map)
});

pub fn insert_parameter(fname: PlatformParameter, fvalue: Arc<dyn Feature>) {
    let mut dict = FEATURE_TOP.lock().unwrap();
    let mut paramlist = PLATFORM_PARAMETERS.lock().unwrap();
    if !paramlist.contains(&fname) {
        paramlist.push(fname.clone());
    }
    dict.insert(fname, fvalue);
}