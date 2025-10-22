use std::{collections::HashMap, sync::{Arc, Mutex}};

use once_cell::sync::Lazy;

use super::Feature;

pub type PlatformParameter = String;


pub static PLATFORM_PARAMETERS: Lazy<Mutex<Vec<PlatformParameter>>> = Lazy::new(|| {
    let map: Vec<PlatformParameter> = Vec::new();
    Mutex::new(map)
});

pub type PlatformFeatures = HashMap<PlatformParameter, Arc<dyn Feature + Send + Sync>>;


pub static FEATURE_TOP: Lazy<Mutex<PlatformFeatures>> = Lazy::new(|| {
    let map: PlatformFeatures = HashMap::new();    
    Mutex::new(map)
});

pub fn insert_parameter(fname:PlatformParameter, fvalue: Arc<dyn Feature + Send + Sync>) {
    let mut dict = FEATURE_TOP.lock().unwrap();
    let mut paramlist = PLATFORM_PARAMETERS.lock().unwrap();
    paramlist.push(fname.clone());
    dict.insert(fname, fvalue);
}