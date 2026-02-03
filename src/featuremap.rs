
use std::{collections::HashMap, sync::Mutex};

use once_cell::sync::Lazy;

use crate::{QualifierFeature};

use std::sync::Arc;

pub static FEATURE_MAP: Lazy<Mutex<HashMap<String, Arc<dyn QualifierFeature>>>> = Lazy::new(|| {
    let map: HashMap<String, Arc<dyn QualifierFeature>> = HashMap::new();
    Mutex::new(map)
});

pub fn insert_feature(fvalue: Arc<dyn QualifierFeature>) {
    let mut dict = FEATURE_MAP.lock().unwrap();
    // println!("insert_feature({})",fvalue.string());
    dict.insert(fvalue.string(), fvalue);

    /*for (key, feature) in &dict.clone() {
        println!("--- {}: {}", key, feature.string());
    }*/

}

pub fn lookup_feature(fname: &'static str) -> Option<Arc<dyn QualifierFeature>> {
    let dict = FEATURE_MAP.lock().unwrap();
/*     println!("LOOKUP FEATURE {fname}");

     for (key, feature) in &dict.clone() {
        println!("+++ {}: {}", key, feature.string());
    }*/

    match dict.get(fname) {
        Some(v) => Some(v.clone()),
        None => None,
    }

//    dict.get(fname).unwrap().clone()
}

