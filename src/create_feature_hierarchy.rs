
/// Macro that generates a hierarchy of features with specified supertypes and associates them with a feature class.
///
/// This macro allows you to define a chain of features where each feature inherits from its predecessor,
/// ultimately linking to a base feature. It also supports defining multiple leaf features that share a common base feature.
///
///
/// # Example
/// ```
/// create_feature_hierarchy! {
///     A -> B -> C => "some_class";
///     X & Y & Z -> Base => "another_class";
/// }
/// ```




#[macro_export]
macro_rules! supertype {
    (None) => { None };
    ($other:ident) => { Some(Box::new($other)) };
}

 

#[macro_export]
macro_rules! create_feature_hierarchy {
    (@chain { $class_name:literal } $name:ident -> $next:ident $( -> $rest:ident )*) => {
        #[derive(Clone)]
        #[allow(non_camel_case_types)]
        pub struct $next;
        
        impl configurable_features::Feature for $next {
            fn feature_type(self:&Self) -> configurable_features::FeatureKind { configurable_features::FeatureKind::Qualifier }
            fn feature_obj(self:&Self) -> configurable_features::FeatureObj { configurable_features::FeatureObj::Qualifier(std::sync::Arc::new(self.clone()) as std::sync::Arc<dyn configurable_features::QualifierFeature>) }
            fn string(&self) -> &'static str { stringify!($next) }
            fn supertype(&self) -> Option<Box<dyn configurable_features::Feature>> { configurable_features::supertype!($name) }
            fn feature_class(&self) -> Option<configurable_features::PlatformParameter> { Some($class_name.to_string()) }
        }

        impl configurable_features::QualifierFeature for $next {}
        create_feature_hierarchy!(@chain { $class_name } $next $( -> $rest )*);
    };

     (@chain { $class_name:literal } $base:ident) => {  };

    ( $tag:ident ; $class_name:literal : $head:ident $( -> $tail:ident )+ ; ) => {
        paste::paste! {
//            use crate::{$head, Feature, configurable_features::FeatureKind, configurable_features::FeatureObj, configurable_features::PlatformParameter, configurable_features::QualifierFeature, configurable_features::insert_feature};
//            use ctor::ctor;
//            use std::sync::Arc;
//            use crate::supertype;

            #[allow(non_snake_case)]
            #[ctor::ctor]
            fn [<$tag>]() {
                $(
                    configurable_features::insert_feature(std::sync::Arc::new($tail));
                )*
            }
        }
        create_feature_hierarchy!(@chain { $class_name } $head $( -> $tail )*);
    };



    ( $tag:ident ; $class_name:literal : $base:ident -> $($leaf:ident)&+; ) => {
        $(
            #[derive(Clone)]
            #[allow(non_camel_case_types)]
            pub struct $leaf;

            impl configurable_features::Feature for $leaf {
                fn feature_type(self:&Self) -> configurable_features::FeatureKind { configurable_features::FeatureKind::Qualifier }
                fn feature_obj(self:&Self) -> configurable_features::FeatureObj { configurable_features::FeatureObj::Qualifier(std::sync::Arc::new(self.clone()) as std::sync::Arc<dyn configurable_features::QualifierFeature>) }
                fn string(&self) -> &'static str { stringify!($leaf) }
                fn supertype(&self) -> Option<Box<dyn configurable_features::Feature>> { Some(Box::new($base)) }
                fn feature_class(&self) -> Option<configurable_features::PlatformParameter> { Some($class_name.to_string()) }
            }

            impl configurable_features::QualifierFeature for $leaf {}
        )*
        paste::paste! {
            #[allow(non_snake_case)]
            #[ctor::ctor]
            fn [<$tag>]() {
                $( configurable_features::insert_feature(std::sync::Arc::new($leaf)); )*
            }
        }
    };
}
