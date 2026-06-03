use serde::Serialize;

pub fn result<T: Serialize>(data: &T) {
    let out = serde_json::json!({ "status": "ok", "data": data });
    println!("{}", serde_json::to_string(&out).unwrap());
}

pub fn error(err: &str) {
    let out = serde_json::json!({ "status": "error", "error": err });
    println!("{}", serde_json::to_string(&out).unwrap());
}
