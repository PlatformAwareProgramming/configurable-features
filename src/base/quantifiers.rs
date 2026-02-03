use std::sync::Arc;

use crate::{Feature, FeatureKind, PlatformParameter, QuantifierFeature};

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
    fn supertype(self:&Self) -> Option<Box<dyn Feature>> {  Some(Box::new(AtLeast { val:self.val+1 }) as Box<dyn Feature>)  }
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
    fn supertype(self:&Self) -> Option<Box<dyn Feature>> {  Some(Box::new(AtMost { val:self.val-1 }) )  }
}


impl QuantifierFeature for AtMost {  
    fn val(self:&Self) -> i32 { (*self).val } 
    fn quantifier_type(self:&Self) -> QuantifierType { QuantifierType::AtMost }
}

impl Feature for i32 {
    fn feature_obj(self:&Self) -> super::FeatureObj { super::FeatureObj::Quantifier(Arc::new(*self)) }
    fn string(self:&Self) -> String { format!("exactly {self}") } 
    
}

impl QuantifierFeature for i32 {  
    fn val(self:&Self) -> i32 { *self } 
    fn quantifier_type(self:&Self) -> QuantifierType { QuantifierType::ExactValue }
}
