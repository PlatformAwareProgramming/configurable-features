use crate::{PlatformParameter, QuantifierType};

use std::{collections::hash_map::DefaultHasher, hash::{Hash, Hasher}, sync::Arc};

pub enum FeatureKind { Qualifier, Quantifier }

pub trait Feature: Send + Sync {
    fn feature_obj(self:&Self) -> FeatureObj;
    fn string(&self) -> String;
    fn supertype(&self) -> Option<Box<dyn Feature>> { None }

    fn hash_code(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.string().hash(&mut s);
        s.finish()
    }
}


pub trait QualifierFeature: Feature {
    fn feature_class(&self) -> PlatformParameter;
}

// Only quantifier features have quantifier data.
pub trait QuantifierFeature: Feature {
    fn quantifier_type(&self) -> QuantifierType;
    fn val(&self) -> i32;
}


pub enum FeatureObj {
    Qualifier(Arc<dyn Feature>),
    QualifierVec(Vec<Arc<dyn QualifierFeature>>),
    Quantifier(Arc<dyn QuantifierFeature>),
}

impl FeatureObj {
    pub fn subtypeof(&self, other: &FeatureObj) -> bool {
        match (self, other) {
            (FeatureObj::Qualifier(a), FeatureObj::Qualifier(b)) => {
                let target = b.string();
                if a.string() == target {
                    return true;
                }
                let mut s = a.supertype();
                while let Some(ref sup) = s {
                    if sup.string() == target {
                        return true;
                    }
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
                match (qt_self, qt_other) {
                    (QuantifierType::AtLeast, QuantifierType::AtLeast) => v_self >= v_other,
                    (QuantifierType::AtMost, QuantifierType::AtMost) => v_self <= v_other,
                    (QuantifierType::ExactValue, QuantifierType::AtLeast) => v_self >= v_other,
                    (QuantifierType::ExactValue, QuantifierType::AtMost) => v_self <= v_other,
                    (QuantifierType::ExactValue, QuantifierType::ExactValue) => v_self == v_other,
                    _ => false,
                }
            }
            // qualifier vs quantifier: not comparable (or define your own rule)
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AtLeast, AtMost};

    #[test]
    fn test_quantifier_subtyping_rules() {
        let al4 = AtLeast { val: 4 }.feature_obj();
        let al8 = AtLeast { val: 8 }.feature_obj();
        let am4 = AtMost { val: 4 }.feature_obj();
        let am8 = AtMost { val: 8 }.feature_obj();
        let ex4 = 4i32.feature_obj();
        let ex8 = 8i32.feature_obj();

        // AtLeast{m} <: AtLeast{n} iff m >= n
        assert!(al8.subtypeof(&al4));
        assert!(al4.subtypeof(&al4));
        assert!(!al4.subtypeof(&al8));

        // AtMost{m} <: AtMost{n} iff m <= n
        assert!(am4.subtypeof(&am8));
        assert!(am4.subtypeof(&am4));
        assert!(!am8.subtypeof(&am4));

        // m <: AtLeast{n} iff m >= n; m <: AtMost{n} iff m <= n; m <: n iff m == n
        assert!(ex8.subtypeof(&al4));
        assert!(ex4.subtypeof(&al4));
        assert!(!ex4.subtypeof(&al8));
        assert!(ex4.subtypeof(&am8));
        assert!(ex4.subtypeof(&am4));
        assert!(!ex8.subtypeof(&am4));
        assert!(ex4.subtypeof(&ex4));
        assert!(!ex4.subtypeof(&ex8));

        // Interval quantifiers are never subtypes of an exact value or of the opposite bound
        assert!(!al4.subtypeof(&ex4));
        assert!(!al8.subtypeof(&ex8));
        assert!(!am4.subtypeof(&ex4));
        assert!(!am4.subtypeof(&ex8));
        assert!(!al4.subtypeof(&am8));
        assert!(!am4.subtypeof(&al4));
    }
}
