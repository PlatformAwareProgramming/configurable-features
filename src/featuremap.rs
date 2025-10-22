
use std::{collections::HashMap, sync::Mutex};

use once_cell::sync::Lazy;

use crate::Feature;

use std::sync::Arc;

static FEATURE_MAP: Lazy<Mutex<HashMap<&'static str, Arc<dyn Feature + Send + Sync>>>> = Lazy::new(|| {
    let map: HashMap<&'static str, Arc<dyn Feature + Send + Sync>> = HashMap::new();
    Mutex::new(map)
});

pub fn insert_feature(fvalue: Arc<dyn Feature + Send + Sync>) {
    let mut dict = FEATURE_MAP.lock().unwrap();
    dict.insert(fvalue.string(), fvalue);
}

pub fn lookup_feature(fname: &'static str) -> Option<Arc<dyn Feature + Send + Sync>> {
    let dict = FEATURE_MAP.lock().unwrap();
 //   println!("{fname}");

    match dict.get(fname) {
        Some(v) => Some(v.clone()),
        None => None,
    }

//    dict.get(fname).unwrap().clone()
}


