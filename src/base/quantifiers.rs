use crate::{FeatureType, Feature, PlatformParameter};

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
    fn is_top(self:&Self) -> bool { self.val==0 }
     fn string(self:&Self) -> &'static str { "atleast" } 
    fn feature_type(self:&Self) -> FeatureType { FeatureType::Quantifier } 
    fn feature_class(self:&Self) -> std::option::Option<PlatformParameter> { None }
    fn val(self:&Self) -> Option<i32> { Some((*self).val) } 
    fn quantifier_type(self:&Self) -> Option<QuantifierType> { Some(QuantifierType::AtLeast) }    
    fn supertype(self:&Self) -> Option<Box<dyn Feature>> {  Some(Box::new(AtLeast { val:self.val+1 }) as Box<dyn Feature>)  }
}

impl Feature for AtMost {  
    fn is_top(self:&Self) -> bool { self.val==std::i32::MAX }
     fn string(self:&Self) -> &'static str { "atmost" } 
    fn feature_type(self:&Self) -> FeatureType { FeatureType::Quantifier } 
    fn feature_class(self:&Self) -> std::option::Option<PlatformParameter> { None }
    fn val(self:&Self) -> Option<i32> { Some((*self).val) } 
    fn quantifier_type(self:&Self) -> Option<QuantifierType> { Some(QuantifierType::AtMost) }
    fn supertype(self:&Self) -> Option<Box<dyn Feature>> {  Some(Box::new(AtMost { val:self.val-1 }) as Box<dyn Feature>)  }
}

impl Feature for i32 {  
    fn is_top(self:&Self) -> bool { false }   // ???
    fn string(self:&Self) -> &'static str { /*self.val().unwrap().to_string().as_str() */ "exact value"} 
    fn feature_type(self:&Self) -> FeatureType { FeatureType::Quantifier } 
    fn feature_class(self:&Self) -> std::option::Option<PlatformParameter> { None }
    fn val(self:&Self) -> Option<i32> { Some(*self) } 
    fn quantifier_type(self:&Self) -> Option<QuantifierType> { Some(QuantifierType::Value) }
    fn supertype(self:&Self) -> Option<Box<dyn Feature>> { None }
}
