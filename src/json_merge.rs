use serde_json::{Map, Value};
use std::collections::HashSet;

pub fn merge_json(base_str: &str, ours_str: &str, theirs_str: &str) -> Option<String> {
    let base_val: Value = serde_json::from_str(base_str).ok()?;
    let ours_val: Value = serde_json::from_str(ours_str).ok()?;
    let theirs_val: Value = serde_json::from_str(theirs_str).ok()?;

    let (base_obj, ours_obj, theirs_obj) = match (base_val.as_object(), ours_val.as_object(), theirs_val.as_object()) {
        (Some(b), Some(o), Some(t)) => (b, o, t),
        _ => return None,
    };

    let merged = merge_json_objects(base_obj, ours_obj, theirs_obj)?;
    serde_json::to_string_pretty(&Value::Object(merged)).ok()
}

fn merge_json_objects(
    base: &Map<String, Value>,
    ours: &Map<String, Value>,
    theirs: &Map<String, Value>,
) -> Option<Map<String, Value>> {
    let mut all_keys = std::collections::BTreeSet::new();
    all_keys.extend(base.keys());
    all_keys.extend(ours.keys());
    all_keys.extend(theirs.keys());

    let mut result = Map::new();

    for key in all_keys {
        let b = base.get(key);
        let o = ours.get(key);
        let t = theirs.get(key);

        match (b, o, t) {
            (Some(bv), Some(ov), Some(tv)) => {
                if ov == tv {
                    result.insert(key.clone(), ov.clone());
                } else if ov == bv {
                    result.insert(key.clone(), tv.clone());
                } else if tv == bv {
                    result.insert(key.clone(), ov.clone());
                } else if ov.is_object() && tv.is_object() && bv.is_object() {
                    let sub_merged = merge_json_objects(
                        bv.as_object()?,
                        ov.as_object()?,
                        tv.as_object()?,
                    )?;
                    result.insert(key.clone(), Value::Object(sub_merged));
                } else if ov.is_array() && tv.is_array() && bv.is_array() {
                    let merged_arr = merge_json_arrays(
                        bv.as_array()?,
                        ov.as_array()?,
                        tv.as_array()?,
                    )?;
                    result.insert(key.clone(), Value::Array(merged_arr));
                } else {
                    return None;
                }
            }
            (None, Some(ov), None) => {
                result.insert(key.clone(), ov.clone());
            }
            (None, None, Some(tv)) => {
                result.insert(key.clone(), tv.clone());
            }
            (None, Some(ov), Some(tv)) => {
                if ov == tv {
                    result.insert(key.clone(), ov.clone());
                } else if ov.is_object() && tv.is_object() {
                    let dummy_base = Map::new();
                    let sub_merged = merge_json_objects(
                        &dummy_base,
                        ov.as_object()?,
                        tv.as_object()?,
                    )?;
                    result.insert(key.clone(), Value::Object(sub_merged));
                } else if ov.is_array() && tv.is_array() {
                    let dummy_base = Vec::new();
                    let merged_arr = merge_json_arrays(
                        &dummy_base,
                        ov.as_array()?,
                        tv.as_array()?,
                    )?;
                    result.insert(key.clone(), Value::Array(merged_arr));
                } else {
                    return None;
                }
            }
            (Some(bv), Some(ov), None) => {
                if ov != bv {
                    return None;
                }
            }
            (Some(bv), None, Some(tv)) => {
                if tv != bv {
                    return None;
                }
            }
            (Some(_), None, None) => {}
            (None, None, None) => {}
        }
    }

    Some(result)
}

fn merge_json_arrays(
    base: &[Value],
    ours: &[Value],
    theirs: &[Value],
) -> Option<Vec<Value>> {
    let is_primitive_array = base.iter().chain(ours).chain(theirs).all(|v| {
        v.is_string() || v.is_number() || v.is_boolean()
    });

    if !is_primitive_array {
        if ours == theirs {
            return Some(ours.to_vec());
        }
        return None;
    }

    let mut result = Vec::new();
    let mut seen = HashSet::new();

    let mut bi = 0;
    let mut oi = 0;
    let mut ti = 0;

    for b_item in base {
        while oi < ours.len() && &ours[oi] != b_item && !base[bi..].contains(&ours[oi]) {
            if seen.insert(ours[oi].to_string()) {
                result.push(ours[oi].clone());
            }
            oi += 1;
        }

        while ti < theirs.len() && &theirs[ti] != b_item && !base[bi..].contains(&theirs[ti]) {
            if seen.insert(theirs[ti].to_string()) {
                result.push(theirs[ti].clone());
            }
            ti += 1;
        }

        let kept_in_ours = ours.contains(b_item);
        let kept_in_theirs = theirs.contains(b_item);

        if kept_in_ours && kept_in_theirs {
            if seen.insert(b_item.to_string()) {
                result.push(b_item.clone());
            }
        }

        if oi < ours.len() && &ours[oi] == b_item {
            oi += 1;
        }
        if ti < theirs.len() && &theirs[ti] == b_item {
            ti += 1;
        }
        bi += 1;
    }

    while oi < ours.len() {
        if seen.insert(ours[oi].to_string()) {
            result.push(ours[oi].clone());
        }
        oi += 1;
    }

    while ti < theirs.len() {
        if seen.insert(theirs[ti].to_string()) {
            result.push(theirs[ti].clone());
        }
        ti += 1;
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_package_json_dependencies_and_arrays() {
        let base = r#"{
            "name": "my-app",
            "keywords": ["react", "web"],
            "dependencies": {
                "react": "^18.0.0"
            }
        }"#;

        let ours = r#"{
            "name": "my-app",
            "keywords": ["react", "web", "frontend"],
            "dependencies": {
                "react": "^18.0.0",
                "lodash": "^4.17.21"
            }
        }"#;

        let theirs = r#"{
            "name": "my-app",
            "keywords": ["react", "web", "typescript"],
            "dependencies": {
                "react": "^18.0.0",
                "axios": "^1.6.0"
            }
        }"#;

        let res = merge_json(base, ours, theirs).unwrap();
        assert!(res.contains("lodash"));
        assert!(res.contains("axios"));
        assert!(res.contains("frontend"));
        assert!(res.contains("typescript"));
    }
}