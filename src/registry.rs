use std::sync::Arc;

use crate::{
    CURRENT_FEATURES, FEATURE_TOP, Feature, PLATFORM_PARAMETERS, PlatformFeatures,
    PlatformParameter, QualifierFeature, insert_feature, insert_parameter, lookup_feature,
};

/// Cohesive facade for managing the global platform model and active feature set.
///
/// Located at the crate top level so that `base`, `featuremap`, and `platformfile`
/// remain acyclic and independent of one another.
pub struct PlatformRegistry;

impl PlatformRegistry {
    /// Registers a platform parameter and its weakest assumption (⊤), avoiding duplicates.
    pub fn register_parameter(name: impl Into<PlatformParameter>, top_feature: Arc<dyn Feature>) {
        insert_parameter(name.into(), top_feature);
    }

    /// Returns a snapshot of all registered platform parameter names.
    pub fn parameters() -> Vec<PlatformParameter> {
        PLATFORM_PARAMETERS.lock().unwrap().clone()
    }

    /// Returns a snapshot of the weakest assumptions (⊤) mapped by parameter name.
    pub fn top_features() -> PlatformFeatures {
        FEATURE_TOP.lock().unwrap().clone()
    }

    /// Registers a qualifier feature in the global feature catalog.
    pub fn register_qualifier(feature: Arc<dyn QualifierFeature>) {
        insert_feature(feature);
    }

    /// Looks up a registered qualifier feature by its string representation.
    pub fn lookup_qualifier(name: &str) -> Option<Arc<dyn QualifierFeature>> {
        lookup_feature(name)
    }

    /// Returns a snapshot of the actual platform feature set (P).
    pub fn current_features() -> PlatformFeatures {
        CURRENT_FEATURES.lock().unwrap().clone()
    }

    /// Replaces the actual platform feature set (P).
    pub fn set_current_features(features: PlatformFeatures) {
        *CURRENT_FEATURES.lock().unwrap() = features;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AtLeast;

    #[test]
    fn test_insert_parameter_is_idempotent_on_parameter_list() {
        let param_name = "__test_idempotent_param__".to_string();
        PlatformRegistry::register_parameter(param_name.clone(), Arc::new(AtLeast { val: 1 }));
        PlatformRegistry::register_parameter(param_name.clone(), Arc::new(AtLeast { val: 2 }));

        let params = PlatformRegistry::parameters();
        let count = params.iter().filter(|p| *p == &param_name).count();
        assert_eq!(count, 1);

        let tops = PlatformRegistry::top_features();
        assert_eq!(tops.get(&param_name).unwrap().string(), "atleast 2");
    }
}
