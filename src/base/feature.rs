//mod quantifier;


use crate::{PlatformParameter, QuantifierType};

use std::{hash::{DefaultHasher, Hash, Hasher}, sync::Arc};

pub enum FeatureType {
    Qualifier,
    Quantifier,
}

pub trait Feature {

    fn is_top(self:&Self) -> bool { false }

    fn string(self:&Self) -> &'static str;

    fn hash_code(self:&Self) -> u64 {
        let mut s = DefaultHasher::new();
        self.string().hash(&mut s);
        s.finish()
    }

    fn feature_class(self:&Self) -> Option<PlatformParameter>;

    fn feature_type(self:&Self) -> FeatureType { FeatureType::Qualifier }

    fn supertype(self:&Self) -> Option<Box<dyn Feature>> { None } 

    fn quantifier_type(self:&Self) -> Option<QuantifierType> { None } // only for quantifiers
    fn val(self:&Self) -> Option<i32> { None }                        // only for quantifiers

    // Subtype relation betwenn qualifier and quantifier features
    fn subtypeof(self:&Self, other:Arc<dyn Feature + Send + Sync>) -> bool {
        if self.feature_class() != other.feature_class() { return false };
        match self.feature_type() {
            FeatureType::Qualifier => {
                if self.hash_code() == other.hash_code() { return true }
                let mut s = self.supertype();
                loop {
                    match s {
                        Some(ref f) => { 
                            if f.hash_code() == other.hash_code() { break } 
                            else { s = s.expect("unexpected error").supertype() }
                        },
                        None => break
                    };                    
                };
                s.is_some()
            },
            FeatureType::Quantifier => {       

                let v_self = self.val().expect("unexpected error");
                let v_other = other.val().expect("unexpected error");

                let qt_self = self.quantifier_type().expect("quantifer expected");
                let qt_other = other.quantifier_type().expect("quantifier expected");
               
                match qt_self {
                    QuantifierType::AtLeast => match qt_other {
                        QuantifierType::AtLeast => v_self >= v_other,
                        QuantifierType::AtMost => false,
                        QuantifierType::Value => v_self >= v_other ,
                    },
                    QuantifierType::AtMost => match qt_other {
                        QuantifierType::AtLeast => false,
                        QuantifierType::AtMost => v_self <= v_other,
                        QuantifierType::Value => v_self <= v_other,
                    },
                    QuantifierType::Value => match qt_other {
                        QuantifierType::AtLeast => v_self >= v_other ,
                        QuantifierType::AtMost => v_self <= v_other,
                        QuantifierType::Value => v_self == v_other,
                    },
                }
            },
        }
    }

}


pub struct NullFeature;

impl Feature for NullFeature {
    fn string(self:&Self) -> &'static str { "null" }
    fn feature_class(self:&Self) -> Option<PlatformParameter> { None }
}


