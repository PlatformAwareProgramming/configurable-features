use std::sync::Arc;

use crate::{Feature, QuantifierFeature};

pub enum QuantifierType {
    AtLeast,
    AtMost,
    ExactValue
}

pub struct AtLeast {
    pub val:i32
}

pub struct AtMost {
    pub val:i32
}


impl Feature for AtLeast {
    fn feature_obj(self:&Self) -> super::FeatureObj { super::FeatureObj::Quantifier(Arc::new(AtLeast { val: self.val })) }
    fn string(self:&Self) -> String { format!("atleast {}", self.val) }
    fn supertype(self:&Self) -> Option<Box<dyn Feature>> {
        self.val
            .checked_sub(1)
            .map(|val| Box::new(AtLeast { val }) as Box<dyn Feature>)
    }
}

impl QuantifierFeature for AtLeast {  
    fn val(self:&Self) -> i32 { (*self).val } 
    fn quantifier_type(self:&Self) -> QuantifierType { QuantifierType::AtLeast }    
}

impl Feature for AtMost {
    fn feature_obj(self:&Self) -> super::FeatureObj {
        super::FeatureObj::Quantifier(Arc::new(AtMost { val: self.val }))
    }
    fn string(self:&Self) -> String { format!("atmost {}", self.val) }
    fn supertype(self:&Self) -> Option<Box<dyn Feature>> {
        self.val
            .checked_add(1)
            .map(|val| Box::new(AtMost { val }) as Box<dyn Feature>)
    }
}


impl QuantifierFeature for AtMost {  
    fn val(self:&Self) -> i32 { (*self).val } 
    fn quantifier_type(self:&Self) -> QuantifierType { QuantifierType::AtMost }
}

impl Feature for i32 {
    fn feature_obj(self:&Self) -> super::FeatureObj { super::FeatureObj::Quantifier(Arc::new(*self)) }
    fn string(self:&Self) -> String { format!("exactly {self}") } 
    fn supertype(self:&Self) -> Option<Box<dyn Feature>> {
        Some(Box::new(AtLeast { val: *self }))
    }
}

impl QuantifierFeature for i32 {  
    fn val(self:&Self) -> i32 { *self } 
    fn quantifier_type(self:&Self) -> QuantifierType { QuantifierType::ExactValue }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantifier_supertype_follows_subtyping_direction_and_bounds() {
        // AtLeast{m} <: AtLeast{m - 1}; weakest possible in i32 is AtLeast{i32::MIN}
        let al8 = AtLeast { val: 8 };
        let al8_sup = al8.supertype().unwrap();
        assert_eq!(al8_sup.string(), "atleast 7");
        assert!(al8.feature_obj().subtypeof(&al8_sup.feature_obj()));
        assert!(!al8_sup.feature_obj().subtypeof(&al8.feature_obj()));
        assert!(AtLeast { val: i32::MIN }.supertype().is_none());

        // AtMost{m} <: AtMost{m + 1}; weakest possible in i32 is AtMost{i32::MAX}
        let am4 = AtMost { val: 4 };
        let am4_sup = am4.supertype().unwrap();
        assert_eq!(am4_sup.string(), "atmost 5");
        assert!(am4.feature_obj().subtypeof(&am4_sup.feature_obj()));
        assert!(!am4_sup.feature_obj().subtypeof(&am4.feature_obj()));
        assert!(AtMost { val: i32::MAX }.supertype().is_none());

        // ExactValue is never a top assumption type (supertype is not None)
        let exact = 4i32;
        let exact_sup = exact.supertype().unwrap();
        assert_eq!(exact_sup.string(), "atleast 4");
        assert!(exact.feature_obj().subtypeof(&exact_sup.feature_obj()));
        assert!(!exact_sup.feature_obj().subtypeof(&exact.feature_obj()));
    }
}
