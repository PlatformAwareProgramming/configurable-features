use std::sync::Arc;

use crate::QualifierFeature;

use super::Feature;

impl Feature for Vec<Arc<dyn QualifierFeature>> {
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
    fn feature_class(self:&Self) -> super::PlatformParameter {
        self.first().map(|f| f.feature_class()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_qualifier_vec_does_not_panic_on_feature_class() {
        let empty: Vec<Arc<dyn QualifierFeature>> = Vec::new();
        assert_eq!(empty.feature_class(), "");
        assert_eq!(empty.string(), "[]");
    }
}