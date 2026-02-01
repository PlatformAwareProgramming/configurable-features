//mod quantifier;


use crate::{PlatformParameter, QuantifierType};

use std::{collections::hash_map::DefaultHasher, hash::{Hash, Hasher}, sync::Arc};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FeatureKind { Qualifier, Quantifier }

pub trait Feature: Send + Sync {
    fn feature_obj(self:&Self) -> FeatureObj;
    fn is_top(&self) -> bool { false }
    fn string(&self) -> &'static str;
    fn feature_class(&self) -> Option<PlatformParameter>;
    fn feature_type(self:&Self) -> FeatureKind;
    fn supertype(&self) -> Option<Box<dyn Feature>> { None }

    fn hash_code(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.string().hash(&mut s);
        s.finish()
    }
}


pub trait QualifierFeature: Feature + Send + Sync {

}

// Only quantifier features have quantifier data.
pub trait QuantifierFeature: Feature + Send + Sync {
    fn quantifier_type(&self) -> QuantifierType;
    fn val(&self) -> i32;
}


//pub struct NullFeature;

/*impl Feature for NullFeature {
    fn string(self:&Self) -> &'static str { "null" }
    fn feature_class(self:&Self) -> Option<PlatformParameter> { None }
}
*/

pub enum FeatureObj {
    Qualifier(Arc<dyn Feature + Send + Sync>),
    QualifierVec(Vec<Arc<dyn Feature + Send + Sync>>),
    Quantifier(Arc<dyn QuantifierFeature + Send + Sync>),

}

impl FeatureObj {
    pub fn feature_class(&self) -> Option<PlatformParameter> {
        match self {
            FeatureObj::Qualifier(f) => f.feature_class(),
            FeatureObj::QualifierVec(f) => f.first().map(|first| first.feature_class()).flatten(),
            FeatureObj::Quantifier(f) => f.feature_class(),
        }
    }

    pub fn subtypeof(&self, other: &FeatureObj) -> bool {
        if self.feature_class() != other.feature_class() { return false; }

        match (self, other) {
            (FeatureObj::Qualifier(a), FeatureObj::Qualifier(b)) => {
                if a.hash_code() == b.hash_code() { return true; }
                let mut s = a.supertype();
                while let Some(ref sup) = s {
                    if sup.hash_code() == b.hash_code() { return true; }
                    s = sup.supertype();
                }
                false
            }
            (FeatureObj::QualifierVec(a), FeatureObj::Qualifier(b)) => {
                for f1 in a.iter() {
                    if FeatureObj::Qualifier(f1.clone()).subtypeof(&FeatureObj::Qualifier(b.clone())) { return true; }
                }
                false
            }
            (FeatureObj::Quantifier(a), FeatureObj::Quantifier(b)) => {
                let v_self = a.val();
                let v_other = b.val();
                let qt_self = a.quantifier_type();
                let qt_other = b.quantifier_type();
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
            }
            // qualifier vs quantifier: not comparable (or define your own rule)
            _ => false,
        }
    }
}
