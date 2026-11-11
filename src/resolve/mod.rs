use std::{collections::HashMap, sync::Arc};

use crate::{Feature, PlatformFeatures, PlatformParameter, PlatformRegistry};

/// Pure resolution algorithm decoupled from global state.
///
/// Evaluates `candidates` in reverse declaration order against `actual_features`
/// over the given `parameters`, returning the index of the strongest compatible
/// candidate (`P <: K_n <: ... <: K_0 <: ⊤`), or `-1` if none is compatible.
pub fn resolve_for_platform(
    actual_features: &PlatformFeatures,
    parameters: &[PlatformParameter],
    candidates: &[PlatformFeatures],
) -> i32 {
    if candidates.is_empty() {
        return -1;
    }

    let mut i: i32 = (candidates.len() - 1).try_into().unwrap();
    let mut current_choice: Option<&PlatformFeatures> = None;
    let mut current_choice_index = -1;

    while i >= 0 {
        while i >= 0
            && !(is_subtype_of_with_params(actual_features, &candidates[i as usize], parameters)
                && (current_choice.is_none()
                    || is_subtype_of_with_params(
                        &candidates[i as usize],
                        current_choice.unwrap(),
                        parameters,
                    )))
        {
            i -= 1;
        }

        if i >= 0 {
            current_choice = Some(&candidates[i as usize]);
            current_choice_index = i;
            i -= 1;
        }
    }

    current_choice_index
}

/// Pure platform-type subtyping check (`lhs <: rhs`) over the given `parameters`.
pub fn is_subtype_of_with_params(
    lhs: &PlatformFeatures,
    rhs: &PlatformFeatures,
    parameters: &[PlatformParameter],
) -> bool {
    for p in parameters {
        let vl = lhs.get(p);
        let vr = rhs.get(p);

        let issubtype = match vl {
            None => match vr {
                Some(vrt) => vrt.supertype().is_none(),
                None => true,
            },
            Some(vlt) => match vr {
                Some(vrt) => vlt.feature_obj().subtypeof(&vrt.feature_obj()),
                None => true,
            },
        };
        if !issubtype {
            return false;
        }
    }

    true
}

// The glorious resolution algorithm
pub fn resolve(featureset_list: Vec<HashMap<PlatformParameter, Arc<dyn Feature>>>) -> i32 {
    let actual_features = PlatformRegistry::current_features();
    let parameters = PlatformRegistry::parameters();

    resolve_for_platform(&actual_features, &parameters, &featureset_list)
}

// check whether the left set of features is a subtype of the right set of features (compatibilty relation)
pub fn issubtypeof(
    lhs: &HashMap<PlatformParameter, Arc<dyn Feature>>,
    rhs: &HashMap<PlatformParameter, Arc<dyn Feature>>,
) -> bool {
    let parameters = PlatformRegistry::parameters();
    is_subtype_of_with_params(lhs, rhs, &parameters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AtLeast, AtMost};

    #[test]
    fn test_resolve_empty_candidates_returns_minus_one_without_underflow() {
        let actual = PlatformFeatures::new();
        let params = vec!["cpu_core_count".to_string()];
        assert_eq!(resolve_for_platform(&actual, &params, &[]), -1);
    }

    #[test]
    fn test_pure_resolve_overlapping_assumptions_prefers_later_declaration() {
        let params = vec![
            "cpu_core_count".to_string(),
            "acc_cudacc".to_string(),
        ];

        // Platform has 8 cores and compute capability 80
        let actual: PlatformFeatures = HashMap::from([
            ("cpu_core_count".to_string(), Arc::new(8i32) as Arc<dyn Feature>),
            ("acc_cudacc".to_string(), Arc::new(80i32) as Arc<dyn Feature>),
        ]);

        let v0: PlatformFeatures = HashMap::new();
        // v1 checks only cpu_core_count >= 4
        let v1: PlatformFeatures = HashMap::from([
            ("cpu_core_count".to_string(), Arc::new(AtLeast { val: 4 }) as Arc<dyn Feature>),
        ]);
        // v2 checks only acc_cudacc >= 70 (incomparable with v1, declared later)
        let v2: PlatformFeatures = HashMap::from([
            ("acc_cudacc".to_string(), Arc::new(AtLeast { val: 70 }) as Arc<dyn Feature>),
        ]);

        // Neither v1 <: v2 nor v2 <: v1
        assert!(!is_subtype_of_with_params(&v1, &v2, &params));
        assert!(!is_subtype_of_with_params(&v2, &v1, &params));

        // When declared [v0, v1, v2], the later declaration (v2 at index 2) is chosen
        let order_a = vec![v0.clone(), v1.clone(), v2.clone()];
        let idx_a = resolve_for_platform(&actual, &params, &order_a) as usize;
        assert_eq!(idx_a, 2);
        assert!(order_a[idx_a].contains_key("acc_cudacc"));

        // When declared [v0, v2, v1], the later declaration (v1 at index 2) is chosen
        let order_b = vec![v0, v2, v1];
        let idx_b = resolve_for_platform(&actual, &params, &order_b) as usize;
        assert_eq!(idx_b, 2);
        assert!(order_b[idx_b].contains_key("cpu_core_count"));
    }

    #[test]
    fn test_pure_resolve_refinement_chain_selects_most_specific() {
        let params = vec!["cpu_core_count".to_string()];
        let actual: PlatformFeatures = HashMap::from([
            ("cpu_core_count".to_string(), Arc::new(16i32) as Arc<dyn Feature>),
        ]);

        let v0: PlatformFeatures = HashMap::new();
        let v1: PlatformFeatures = HashMap::from([
            ("cpu_core_count".to_string(), Arc::new(AtLeast { val: 4 }) as Arc<dyn Feature>),
        ]);
        let v2: PlatformFeatures = HashMap::from([
            ("cpu_core_count".to_string(), Arc::new(AtLeast { val: 16 }) as Arc<dyn Feature>),
        ]);
        let v3: PlatformFeatures = HashMap::from([
            ("cpu_core_count".to_string(), Arc::new(AtLeast { val: 8 }) as Arc<dyn Feature>),
        ]);
        let v4: PlatformFeatures = HashMap::from([
            ("cpu_core_count".to_string(), Arc::new(AtMost { val: 4 }) as Arc<dyn Feature>),
        ]);

        // Even though v3 (AtLeast 8) is declared after v2 (AtLeast 16),
        // v2 <: v3, so the algorithm refines the choice from v3 to v2 (index 2)!
        let candidates = vec![v0, v1, v2, v3, v4];
        assert_eq!(resolve_for_platform(&actual, &params, &candidates), 2);
    }
}

