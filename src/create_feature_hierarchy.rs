use crate::PlatformParameter;
use crate::Feature;
use crate::insert_feature;
use ctor::ctor;

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
macro_rules! create_feature_hierarchy {
    (@chain { $class_name:literal } $name:ident -> $next:ident $( -> $rest:ident )*) => {
        pub struct $name;
        impl Feature for $name {
            fn string(&self) -> &'static str { stringify!($name) }
            fn supertype(&self) -> Option<Box<dyn Feature>> { Some(Box::new($next)) }
            fn feature_class(&self) -> Option<PlatformParameter> { Some($class_name.to_string()) }
        }
        create_feature_hierarchy!(@chain { $class_name } $next $( -> $rest )*);
    };

    (@chain { $class_name:literal } $base:ident) => {
        pub struct $base;
        impl Feature for $base {
            fn string(&self) -> &'static str { stringify!($base) }
            fn supertype(&self) -> Option<Box<dyn Feature>> { None }
            fn feature_class(&self) -> Option<PlatformParameter> { Some($class_name.to_string()) }
        }
    };

    ( $head:ident $( -> $tail:ident )+ => $class_name:literal; ) => {
        paste::paste! {
            #[allow(non_snake_case)]
            #[ctor]
            fn [<register_hierarchy_ $head>]() {
                insert_feature(std::sync::Arc::new($head));
                $(
                    insert_feature(std::sync::Arc::new($tail));
                )*
            }
        }
        create_feature_hierarchy!(@chain { $class_name } $head $( -> $tail )*);
    };

    ( $($leaf:ident)&+ -> $base:ident => $class_name:literal; ) => {
        pub struct $base;
        impl Feature for $base {
            fn string(&self) -> &'static str { stringify!($base) }
            fn supertype(&self) -> Option<Box<dyn Feature>> { None }
            fn feature_class(&self) -> Option<PlatformParameter> { Some($class_name.to_string()) }
        }
        $(
            pub struct $leaf;
            impl Feature for $leaf {
                fn string(&self) -> &'static str { stringify!($leaf) }
                fn supertype(&self) -> Option<Box<dyn Feature>> { Some(Box::new($base)) }
                fn feature_class(&self) -> Option<PlatformParameter> { Some($class_name.to_string()) }
            }
        )*
        paste::paste! {
            #[allow(non_snake_case)]
            #[ctor]
            fn [<register_hierarchy_ $base>]() {
                $( insert_feature(std::sync::Arc::new($leaf)); )*
                insert_feature(std::sync::Arc::new($base));
            }
        }
    };
}
