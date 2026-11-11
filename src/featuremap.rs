
use std::{collections::HashMap, sync::{Arc, LazyLock, Mutex}};

use crate::QualifierFeature;

pub static FEATURE_MAP: LazyLock<Mutex<HashMap<String, Arc<dyn QualifierFeature>>>> = LazyLock::new(|| {
    let map: HashMap<String, Arc<dyn QualifierFeature>> = HashMap::new();
    Mutex::new(map)
});

pub fn insert_feature(fvalue: Arc<dyn QualifierFeature>) {
    let mut dict = FEATURE_MAP.lock().unwrap();
    dict.insert(fvalue.string(), fvalue);
}

pub fn lookup_feature(fname: &str) -> Option<Arc<dyn QualifierFeature>> {
    let dict = FEATURE_MAP.lock().unwrap();
    dict.get(fname).cloned()
}

