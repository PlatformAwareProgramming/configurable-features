use std::{hash::{Hash, Hasher}, sync::Arc};

use crate::QualifierFeature;

use super::Feature;

impl Feature for Vec<Arc<dyn QualifierFeature>> {
   // fn is_top(self:&Self) -> bool { false }

    fn hash_code(self:&Self) -> u64 {
        let mut s = std::hash::DefaultHasher::new();
        self.string().hash(&mut s);
        s.finish()
    }

   // fn feature_type(self:&Self) -> super::FeatureKind { super::FeatureKind::Qualifier }

    fn supertype(self:&Self) -> Option<Box<dyn Feature>> { None }

    fn string(&self) -> String {
        let mut s = String::from("[");
        for f in self {
            let fs = f.string();
            s.push_str(&fs);
            s.push(',');
        }
        s.push(']');
    
        s
    }

    
    fn feature_obj(self:&Self) -> super::FeatureObj {
        super::FeatureObj::QualifierVec(self.clone())
    }


}

impl QualifierFeature for Vec<Arc<dyn QualifierFeature>> {
    fn feature_class(self:&Self) -> super::PlatformParameter { self[0].feature_class() }
    
}