use std::{collections::HashMap, sync::Arc};

use crate::{Feature, PlatformFeatures, PlatformParameter, PlatformRegistry};

/// Pure resolution algorithm decoupled from global state.
///
/// Evaluates `candidates` in reverse declaration order against `actual_features`
/// over the given `parameters` and weakest assumptions (`top_features`), returning
/// the index of the strongest compatible candidate (`P <: K_n <: ... <: K_0 <: ⊤`),
/// or `-1` if none is compatible.
pub fn resolve_for_platform_with_tops(
    actual_features: &PlatformFeatures,
    parameters: &[PlatformParameter],
    top_features: &PlatformFeatures,
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
            && !(is_subtype_of_with_tops(
                actual_features,
                &candidates[i as usize],
                parameters,
                top_features,
            ) && match current_choice {
                None => true,
                Some(choice) => {
                    is_subtype_of_with_tops(
                        &candidates[i as usize],
                        choice,
                        parameters,
                        top_features,
                    ) && !is_subtype_of_with_tops(
                        choice,
                        &candidates[i as usize],
                        parameters,
                        top_features,
                    )
                }
            })
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

/// Pure resolution algorithm without an explicit weakest-assumption map.
pub fn resolve_for_platform(
    actual_features: &PlatformFeatures,
    parameters: &[PlatformParameter],
    candidates: &[PlatformFeatures],
) -> i32 {
    let empty_tops = PlatformFeatures::new();
    resolve_for_platform_with_tops(actual_features, parameters, &empty_tops, candidates)
}

/// Pure platform-type subtyping check (`lhs <: rhs`) over `parameters`, substituting
/// each omitted parameter on `lhs` with its weakest assumption from `top_features`.
pub fn is_subtype_of_with_tops(
    lhs: &PlatformFeatures,
    rhs: &PlatformFeatures,
    parameters: &[PlatformParameter],
    top_features: &PlatformFeatures,
) -> bool {
    for p in parameters {
        let vl = lhs.get(p);
        let vr = rhs.get(p);

        let issubtype = match (vl, vr) {
            (None, None) | (Some(_), None) => true,
            (None, Some(vrt)) => match top_features.get(p) {
                Some(top) => top.feature_obj().subtypeof(&vrt.feature_obj()),
                None => vrt.supertype().is_none(),
            },
            (Some(vlt), Some(vrt)) => vlt.feature_obj().subtypeof(&vrt.feature_obj()),
        };
        if !issubtype {
            return false;
        }
    }

    true
}

/// Pure platform-type subtyping check (`lhs <: rhs`) over the given `parameters`.
pub fn is_subtype_of_with_params(
    lhs: &PlatformFeatures,
    rhs: &PlatformFeatures,
    parameters: &[PlatformParameter],
) -> bool {
    let empty_tops = PlatformFeatures::new();
    is_subtype_of_with_tops(lhs, rhs, parameters, &empty_tops)
}

// The glorious resolution algorithm
pub fn resolve(featureset_list: Vec<HashMap<PlatformParameter, Arc<dyn Feature>>>) -> i32 {
    let actual_features = PlatformRegistry::current_features();
    let parameters = PlatformRegistry::parameters();
    let top_features = PlatformRegistry::top_features();

    resolve_for_platform_with_tops(&actual_features, &parameters, &top_features, &featureset_list)
}

// check whether the left set of features is a subtype of the right set of features (compatibilty relation)
pub fn issubtypeof(
    lhs: &HashMap<PlatformParameter, Arc<dyn Feature>>,
    rhs: &HashMap<PlatformParameter, Arc<dyn Feature>>,
) -> bool {
    let parameters = PlatformRegistry::parameters();
    let top_features = PlatformRegistry::top_features();
    is_subtype_of_with_tops(lhs, rhs, &parameters, &top_features)
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

    #[test]
    fn test_exact_value_candidate_not_overridden_by_fallback_or_missing_feature() {
        let params = vec!["acc_count".to_string()];
        let tops: PlatformFeatures = HashMap::from([(
            "acc_count".to_string(),
            Arc::new(AtLeast { val: 0 }) as Arc<dyn Feature>,
        )]);

        let v0_fallback: PlatformFeatures = HashMap::new();
        let v1_exact: PlatformFeatures = HashMap::from([(
            "acc_count".to_string(),
            Arc::new(2i32) as Arc<dyn Feature>,
        )]);
        let candidates = vec![v0_fallback.clone(), v1_exact.clone()];

        // Fallback (⊤) must NOT be a subtype of an exact-value assumption
        assert!(!is_subtype_of_with_tops(&v0_fallback, &v1_exact, &params, &tops));
        assert!(!is_subtype_of_with_params(&v0_fallback, &v1_exact, &params));

        // Platform with acc_count = 2 must select v1_exact (index 1), not fallback
        let actual_two: PlatformFeatures = HashMap::from([(
            "acc_count".to_string(),
            Arc::new(2i32) as Arc<dyn Feature>,
        )]);
        assert_eq!(
            resolve_for_platform_with_tops(&actual_two, &params, &tops, &candidates),
            1
        );
        assert_eq!(resolve_for_platform(&actual_two, &params, &candidates), 1);

        // Platform omitting acc_count must NOT match v1_exact; it must select fallback (index 0)
        let actual_missing: PlatformFeatures = HashMap::new();
        assert_eq!(
            resolve_for_platform_with_tops(&actual_missing, &params, &tops, &candidates),
            0
        );
        assert_eq!(resolve_for_platform(&actual_missing, &params, &candidates), 0);
    }

    #[test]
    fn test_omitted_parameter_satisfies_explicit_weakest_assumption_from_top_features() {
        let params = vec!["acc_count".to_string(), "acc_tdp".to_string()];
        let tops: PlatformFeatures = HashMap::from([
            (
                "acc_count".to_string(),
                Arc::new(AtLeast { val: 0 }) as Arc<dyn Feature>,
            ),
            (
                "acc_tdp".to_string(),
                Arc::new(AtMost { val: i32::MAX }) as Arc<dyn Feature>,
            ),
        ]);

        let actual_omitted: PlatformFeatures = HashMap::new();
        let v0_fallback: PlatformFeatures = HashMap::new();
        let v1_explicit_weakest: PlatformFeatures = HashMap::from([
            (
                "acc_count".to_string(),
                Arc::new(AtLeast { val: 0 }) as Arc<dyn Feature>,
            ),
            (
                "acc_tdp".to_string(),
                Arc::new(AtMost { val: i32::MAX }) as Arc<dyn Feature>,
            ),
        ]);
        let v2_stronger: PlatformFeatures = HashMap::from([(
            "acc_count".to_string(),
            Arc::new(AtLeast { val: 1 }) as Arc<dyn Feature>,
        )]);

        assert!(is_subtype_of_with_tops(
            &actual_omitted,
            &v1_explicit_weakest,
            &params,
            &tops
        ));
        assert!(!is_subtype_of_with_tops(
            &actual_omitted,
            &v2_stronger,
            &params,
            &tops
        ));

        // v1 has the same platform type as v0 (⊤) and is declared later, so v1 keeps precedence
        let candidates = vec![v0_fallback, v1_explicit_weakest, v2_stronger];
        assert_eq!(
            resolve_for_platform_with_tops(&actual_omitted, &params, &tops, &candidates),
            1
        );
    }

    #[test]
    fn test_atmost_and_atleast_not_subtypes_of_exact_value_during_resolution() {
        let params = vec!["cpu_core_count".to_string()];
        let tops: PlatformFeatures = HashMap::from([(
            "cpu_core_count".to_string(),
            Arc::new(AtLeast { val: 1 }) as Arc<dyn Feature>,
        )]);
        let actual_four: PlatformFeatures = HashMap::from([(
            "cpu_core_count".to_string(),
            Arc::new(4i32) as Arc<dyn Feature>,
        )]);

        let v0_fallback: PlatformFeatures = HashMap::new();
        let v1_atmost4: PlatformFeatures = HashMap::from([(
            "cpu_core_count".to_string(),
            Arc::new(AtMost { val: 4 }) as Arc<dyn Feature>,
        )]);
        let v2_atleast4: PlatformFeatures = HashMap::from([(
            "cpu_core_count".to_string(),
            Arc::new(AtLeast { val: 4 }) as Arc<dyn Feature>,
        )]);
        let v3_exact4: PlatformFeatures = HashMap::from([(
            "cpu_core_count".to_string(),
            Arc::new(4i32) as Arc<dyn Feature>,
        )]);

        // Exact 4 is strictly stronger than both AtMost{4} and AtLeast{4};
        // neither AtMost{4} nor AtLeast{4} may override Exact 4.
        let candidates = vec![v0_fallback, v1_atmost4, v2_atleast4, v3_exact4];
        assert_eq!(
            resolve_for_platform_with_tops(&actual_four, &params, &tops, &candidates),
            3
        );
    }
}

