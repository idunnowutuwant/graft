use serde_json::{Map, Value};

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