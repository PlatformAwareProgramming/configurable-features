use std::{hash::{Hash, Hasher}, sync::Arc};

use crate::QualifierFeature;

use super::Feature;

impl Feature for Vec<Arc<dyn Feature + Send + Sync>> {
    fn is_top(self:&Self) -> bool { false }

    fn hash_code(self:&Self) -> u64 {
        let mut s = std::hash::DefaultHasher::new();
        self.string().hash(&mut s);
        s.finish()
    }

    fn feature_type(self:&Self) -> super::FeatureKind { super::FeatureKind::Qualifier }

    fn supertype(self:&Self) -> Option<Box<dyn Feature>> { None }

    fn string(&self) -> &'static str {
        let mut s = String::from("[");
        for f in self {
            let fs = f.string();
            s.push_str(&fs);
            s.push(',');
        }
        s.push(']');
    
        Box::leak(s.into_boxed_str())
    }

    fn feature_class(self:&Self) -> Option<super::PlatformParameter> { self[0].feature_class() }
    
    fn feature_obj(self:&Self) -> super::FeatureObj {
        super::FeatureObj::QualifierVec(self.clone())
    }


}

impl QualifierFeature for Vec<Arc<dyn Feature + Send + Sync>> {


/*     fn subtypeof(self:&Self, other:std::sync::Arc<dyn Feature + Send + Sync>) -> bool {
        if self.feature_class() != other.feature_class() { return false };
        match other.feature_type() {
            FeatureKind::Qualifier => {
                if self.hash_code() == other.hash_code() { return true }

                let v1 = self;
                let v2 = other;

                let mut r: bool = false;
                for f in v1 {
                    if f.subtypeof(v2.clone()) { r = true; break; }
                }

                return r
            },
            FeatureKind::Quantifier => { false }
        }
    }
   */ 
    
    
}