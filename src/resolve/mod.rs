use std::{collections::HashMap, sync::Arc};

use crate::{CURRENT_FEATURES, Feature, PLATFORM_PARAMETERS, PlatformParameter, featuremap::FEATURE_MAP};

// The glorious resolution algorithm
pub fn resolve(featureset_list:Vec<HashMap<PlatformParameter, Arc<dyn Feature>>> ) -> i32
{
    println!("ENTER RESOLVE");

    let actualplatformfeatures: HashMap<PlatformParameter, Arc<dyn Feature>> = CURRENT_FEATURES.lock().unwrap().clone();

     println!("Tamanho da tabela: {}", actualplatformfeatures.len());
    for (key, feature) in &actualplatformfeatures {
        println!("{}: {}", key, feature.string());
    }

        let dict = FEATURE_MAP.lock().unwrap();
     for (key, feature) in &dict.clone() {
        println!("+++ {}: {}", key, feature.string());
    }


    // i points to the current candidate in the featureset_list
    let mut i: i32 = (featureset_list.len()-1).try_into().unwrap();
    let mut current_choice: Option<&HashMap<PlatformParameter,Arc<dyn Feature>>> = None;
    let mut current_choice_index  = -1;

    while i >= 0 {
        // look for the next candidate that is compatible with actualplatformfeatures and is more specific than current_choice, if it is defined
        while i >= 0 && !(issubtypeof(&actualplatformfeatures, &featureset_list[i as usize]) 
                                   && (current_choice.is_none() || 
                                       issubtypeof(&featureset_list[i as usize], &current_choice.unwrap()))) { 
            i -= 1;
        }

        if i >= 0 {
            // if i >= 0, we found a candidate and current choice must be updated
            current_choice = Some(&featureset_list[i as usize]);
            current_choice_index = i;
            i -= 1;
        }
    }

    println!("EXIT RESOLVE {}", current_choice_index);

    current_choice_index
}
    

// check whether the left set of features is a subtype of the right set of features (compatibilty relation)
pub fn issubtypeof(lhs: &HashMap<PlatformParameter, Arc<dyn Feature>>, rhs: &HashMap<PlatformParameter, Arc<dyn Feature>>) -> bool {

    println!("subtype test"); // DEBUG

    for p in PLATFORM_PARAMETERS.lock().unwrap().iter()  {

        print!("subtype test - parameter {:?}", p); // DEBUG
        
        let vl = lhs.get(p);
        let vr = rhs.get(p);

        // begin DEBUG
          match vl {
            Some(s) => print!(" {}", s.string()),
            None => print!(" {}", "no"),
        };

        match vr {
            Some(s) => print!(" {}", s.string()),
            None => print!(" {}", "no"),
        };  
        // end DEBUG

        let issubtype = match vl {
                                None => match vr {
                                    Some(vrt) => vrt.supertype().is_none(),
                                    None => true,
                                }
                                Some(vlt) => match vr {
                                    Some(vrt) => vlt.feature_obj().subtypeof(&vrt.feature_obj()),
                                    None => true, 
                                }
                              };
        println!(" {}", issubtype); // DEBUG
        if !issubtype { return false; }
    } 

    true
}

