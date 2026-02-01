use std::sync::Arc;

use crate::{Feature, FeatureKind, PlatformParameter, QuantifierFeature};

pub enum QuantifierType {
    AtLeast,
    AtMost,
    Value
}

pub struct AtLeast {
    pub val:i32
}

pub struct AtMost {
    pub val:i32
}


impl Feature for AtLeast {
    fn feature_obj(self:&Self) -> super::FeatureObj { super::FeatureObj::Quantifier(Arc::new(AtLeast { val: self.val })) }
    fn is_top(self:&Self) -> bool { self.val==0 }
    fn string(self:&Self) -> &'static str { "atleast" } 
    fn feature_class(self:&Self) -> std::option::Option<PlatformParameter> { None }
    fn feature_type(self:&Self) -> FeatureKind { FeatureKind::Quantifier }
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
    fn is_top(self:&Self) -> bool { self.val==std::i32::MAX }
    fn string(self:&Self) -> &'static str { "atmost" } 
    fn feature_class(self:&Self) -> std::option::Option<PlatformParameter> { None }
    fn feature_type(self:&Self) -> FeatureKind { FeatureKind::Quantifier }
    fn supertype(self:&Self) -> Option<Box<dyn Feature>> {  Some(Box::new(AtMost { val:self.val-1 }) as Box<dyn Feature>)  }
}


impl QuantifierFeature for AtMost {  
    fn val(self:&Self) -> i32 { (*self).val } 
    fn quantifier_type(self:&Self) -> QuantifierType { QuantifierType::AtMost }
}

impl Feature for i32 {
    fn feature_obj(self:&Self) -> super::FeatureObj { super::FeatureObj::Quantifier(Arc::new(*self)) }
    fn is_top(self:&Self) -> bool { false }   // ???
    fn string(self:&Self) -> &'static str { /*self.val().unwrap().to_string().as_str() */ "exact value"} 
    fn feature_type(self:&Self) -> FeatureKind { FeatureKind::Quantifier } 
    fn feature_class(self:&Self) -> std::option::Option<PlatformParameter> { None }
    fn supertype(self:&Self) -> Option<Box<dyn Feature>> { None }
    
}

impl QuantifierFeature for i32 {  
    fn val(self:&Self) -> i32 { *self } 
    fn quantifier_type(self:&Self) -> QuantifierType { QuantifierType::Value }
}
