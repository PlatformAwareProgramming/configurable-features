use std::sync::Arc;

use crate::{
    CURRENT_FEATURES, FEATURE_TOP, Feature, PLATFORM_PARAMETERS, PlatformFeatures,
    PlatformParameter, QualifierFeature, ensure_features_loaded, insert_feature, insert_parameter,
    lookup_feature, platformfile::mark_features_loaded, register_feature_loader,
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

    /// Registers a deferred loader to populate `CURRENT_FEATURES` on first access.
    pub fn register_feature_loader<F>(loader: F)
    where
        F: Fn() -> PlatformFeatures + Send + Sync + 'static,
    {
        register_feature_loader(loader);
    }

    /// Returns a snapshot of the actual platform feature set (P), running deferred
    /// feature loaders on first access so all crate `#[ctor]` registrations are visible.
    pub fn current_features() -> PlatformFeatures {
        ensure_features_loaded();
        CURRENT_FEATURES.lock().unwrap().clone()
    }

    /// Replaces the actual platform feature set (P).
    pub fn set_current_features(features: PlatformFeatures) {
        mark_features_loaded();
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

    #[test]
    fn test_deferred_feature_loader_sees_qualifiers_registered_after_loader() {
        use std::collections::HashMap;
        use crate::{FeatureObj, add_qualifier};

        #[derive(Clone)]
        struct LateVendorFeature;

        impl Feature for LateVendorFeature {
            fn feature_obj(&self) -> FeatureObj {
                FeatureObj::Qualifier(Arc::new(self.clone()))
            }
            fn string(&self) -> String {
                "__late_vendor_feature__".to_string()
            }
        }

        impl QualifierFeature for LateVendorFeature {
            fn feature_class(&self) -> PlatformParameter {
                "__late_vendor_param__".to_string()
            }
        }

        // Simulate core crate registering its Platform.toml loader before a vendor crate's #[ctor]
        PlatformRegistry::register_feature_loader(|| {
            let mut m = HashMap::new();
            add_qualifier(
                &mut m,
                "__late_vendor_param__".to_string(),
                "__late_vendor_feature__".to_string(),
            );
            m
        });

        // Simulate vendor/third-party crate registering its qualifier afterwards
        PlatformRegistry::register_qualifier(Arc::new(LateVendorFeature));

        // First access to current_features() executes the deferred loader after all registrations
        let features = PlatformRegistry::current_features();
        assert_eq!(
            features
                .get("__late_vendor_param__")
                .expect("deferred loader should resolve qualifier registered afterwards")
                .string(),
            "__late_vendor_feature__"
        );
    }
}
