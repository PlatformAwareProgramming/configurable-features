
/// Macro that generates a hierarchy of features with specified supertypes and associates them with a feature class.
///
/// This macro allows you to define a chain of features where each feature inherits from its predecessor,
/// ultimately linking to a base feature. It also supports defining multiple leaf features that share a common base feature.
///
///
/// # Example
/// ```
/// create_feature_hierarchy!{register_features_vendor ;"acc_model" : None :> ACCModel :> NVIDIA_GPU & 
///                                                                                       AMD_GPU & 
///                                                                                       Intel_GPU; 
///                          }                                                  
/// create_feature_hierarchy!{register_features_arch ;"acc_model" : NVIDIA_GPU :> NVIDIA_GPU_Blackwell & 
///                                                                               NVIDIA_GPU_Ada & 
///                                                                               NVIDIA_GPU_Hopper; 
///                          }
/// create_feature_hierarchy!{register_features_model ;"acc_model" : NVIDIA_GPU_Ada :> NVIDIA_GPU_A100 & 
///                                                                                    NVIDIA_GPU_A200 & 
///                                                                                    NVIDIA_GPU_RTX4090 & 
///                                                                                    NVIDIA_GPU_RTX4000; 
/// ```

#[macro_export]
macro_rules! supertype {
    (None) => { None };
    ($other:ident) => { Some(Box::new($other)) };
}

#[macro_export]
macro_rules! create_feature_hierarchy {
    (@chain { $class_name:literal } $name:ident :> $next:ident $( :> $rest:ident )*) => {
        #[derive(Clone)]
        #[allow(non_camel_case_types)]
        pub struct $next;
        
        impl configurable_features::Feature for $next {
          //  fn feature_type(self:&Self) -> configurable_features::FeatureKind { configurable_features::FeatureKind::Qualifier }
            fn feature_obj(self:&Self) -> configurable_features::FeatureObj { configurable_features::FeatureObj::Qualifier(std::sync::Arc::new(self.clone()) ) }
            fn string(&self) -> String { stringify!($next).to_string() }
            fn supertype(&self) -> Option<Box<dyn configurable_features::Feature>> { configurable_features::supertype!($name) }
        }

        impl configurable_features::QualifierFeature for $next {
            fn feature_class(&self) -> configurable_features::PlatformParameter { $class_name.to_string() }
        }

        create_feature_hierarchy!(@chain { $class_name } $next $( :> $rest )*);
    };

     (@chain { $class_name:literal } $base:ident) => {  };

    ( $tag:ident ; $class_name:literal : $head:ident $( :> $tail:ident )+ ; ) => {
        paste::paste! {
            #[allow(non_snake_case)]
            #[ctor::ctor]
            fn [<$tag>]() {
                $(
                    configurable_features::insert_feature(std::sync::Arc::new($tail));
                )*
            }
        }
        create_feature_hierarchy!(@chain { $class_name } $head $( :> $tail )*);
    };



    ( $tag:ident ; $class_name:literal : $base:ident :> $($leaf:ident)&+; ) => {
        $(
            #[derive(Clone)]
            #[allow(non_camel_case_types)]
            pub struct $leaf;

            impl configurable_features::Feature for $leaf {
               // fn feature_type(self:&Self) -> configurable_features::FeatureKind { configurable_features::FeatureKind::Qualifier }
                fn feature_obj(self:&Self) -> configurable_features::FeatureObj { configurable_features::FeatureObj::Qualifier(std::sync::Arc::new(self.clone()) as std::sync::Arc<dyn configurable_features::QualifierFeature>) }
                fn string(&self) -> String { stringify!($leaf).to_string() }
                fn supertype(&self) -> Option<Box<dyn configurable_features::Feature>> { Some(Box::new($base)) }
            }

            impl configurable_features::QualifierFeature for $leaf {
                fn feature_class(&self) -> configurable_features::PlatformParameter { $class_name.to_string() }
            }
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
