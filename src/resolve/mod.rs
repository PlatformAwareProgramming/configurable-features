use std::{collections::HashMap, sync::Arc};

use crate::{Feature, PlatformParameter, CURRENT_FEATURES, PLATFORM_PARAMETERS};

// The glorious resolution algorithm
pub fn resolve(featureset_list:Vec<HashMap<PlatformParameter, Arc<dyn Feature + Send + Sync>>> ) -> i32
{
   // println!("ENTER RESOLVE");

    let actualplatformfeatures: HashMap<PlatformParameter, Arc<dyn Feature + Send + Sync>> = CURRENT_FEATURES.lock().unwrap().clone();

    let mut i: i32 = (featureset_list.len()-1).try_into().unwrap();
    let mut current_choice: Option<&HashMap<PlatformParameter,Arc<dyn Feature + Send + Sync>>> = None;
    let mut current_choice_index  = -1;

    loop {
        while i >= 0 && !(issubtypeof(&actualplatformfeatures, &featureset_list[i as usize]) 
                                   && (current_choice.is_none() || 
                                       issubtypeof(&featureset_list[i as usize], &current_choice.unwrap()))) { 
            i -= 1;
        }

        if i < 0 { break }

        current_choice = Some(&featureset_list[i as usize]);
        current_choice_index = i;
        i -= 1;
    }

    // println!("EXIT RESOLVE {}", current_choice_index + 1);

    current_choice_index + 1
}
    

// check whether the left set of features is a subtype of the right set of features (compatibilty relation)
pub fn issubtypeof(lhs: &HashMap<PlatformParameter, Arc<dyn Feature + Send + Sync>>, rhs: &HashMap<PlatformParameter, Arc<dyn Feature + Send + Sync>>) -> bool {

    //println!("subtype test"); // DEBUG

    for p in PLATFORM_PARAMETERS.lock().unwrap().iter()  {

      //  print!("subtype test - parameter {:?}", p); // DEBUG
        
        let vl = lhs.get(p);
        let vr = rhs.get(p);

        // begin DEBUG
       /*  match vl {
            Some(s) => print!(" {}", s.string()),
            None => print!(" {}", "no"),
        };

        match vr {
            Some(s) => print!(" {}", s.string()),
            None => print!(" {}", "no"),
        }; */ 
        // end DEBUG

        let issubtype = match vl {
                                None => match vr {
                                    Some(vrt) => vrt.is_top(),
                                    None => true,
                                }
                                Some(vlt) => match vr {
                                    Some(vrt) => vlt.subtypeof(vrt.clone()),
                                    None => true, 
                                }
                              };
        //println!(" {}", issubtype); // DEBUG
        if !issubtype { return false; }
    } 

    true
}