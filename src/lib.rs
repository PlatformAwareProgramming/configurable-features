mod base; 
mod platformfile;
mod featuremap;
mod registry;
mod resolve;

pub mod create_feature_hierarchy; 

#[doc(hidden)]
pub mod __private {
    pub use ctor;
    pub use paste;
}

pub use base::{
    AtLeast, AtMost, FEATURE_TOP, Feature, FeatureKind, FeatureObj,
    PLATFORM_PARAMETERS, PlatformFeatures, PlatformParameter, QualifierFeature,
    QuantifierFeature, QuantifierType, insert_parameter,
};
pub use registry::PlatformRegistry;
pub use platformfile::{
    CURRENT_FEATURES, add_qualifier, add_quantifier, readplatform,
};
pub use resolve::{
    is_subtype_of_with_params, issubtypeof, resolve, resolve_for_platform,
};
pub use featuremap::{insert_feature, lookup_feature};
pub use configurable_macros::configurable;
pub use configurable_internal::__internal_configurable;
