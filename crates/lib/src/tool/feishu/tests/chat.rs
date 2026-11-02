use super::*;
use serde_json::json;

fn get() -> Get {
    Get {
        chat: "oc_fixture".into(),
    }
}

#[test]
fn request() {
    let call = get().call().unwrap();
    assert_eq!(call.method, "GET");
    assert_eq!(call.path, "im/v1/chats/oc_fixture?user_id_type=open_id");
    assert!(call.body.is_none());
    for chat in ["oc_", "oc_a?x=y", "ou_other", "oc_a/other"] {
        assert!(Get { chat: chat.into() }.validate().is_err());
    }
}

#[test]
fn scope() {
    let value = json!({"data":{"name":"研发与运维 🐱","chat_mode":"group","chat_type":"private","external":false}});
    let observed = get().decode(&value).unwrap();
    assert_eq!(observed.requested, "oc_fixture");
    assert_eq!(observed.metadata.name.as_deref(), Some("研发与运维 🐱"));
    assert_eq!(observed.metadata.external, Some(false));
    for echo in [json!("oc_foreign"), json!(null), json!(7)] {
        let mut foreign = value.clone();
        foreign["data"]["chat_id"] = echo;
        assert!(get().decode(&foreign).is_err());
    }
    let mut echoed = value;
    echoed["data"]["chat_id"] = json!("oc_fixture");
    assert!(get().decode(&echoed).is_ok());
}

#[test]
fn unknown() {
    let partial = get().decode(&json!({"data":{}})).unwrap();
    assert_eq!(partial.metadata.name, None);
    assert_eq!(partial.metadata.external, None);
    assert_eq!(partial.metadata.mode, None);
    assert_eq!(partial.metadata.kind, None);
    let empty = get()
        .decode(&json!({"data":{"name":"","external":true}}))
        .unwrap();
    assert_eq!(empty.metadata.name.as_deref(), Some(""));
    assert_eq!(empty.metadata.external, Some(true));
    for data in [
        json!(null),
        json!([]),
        json!({"name":3}),
        json!({"external":"false"}),
        json!({"chat_mode":true}),
    ] {
        assert!(get().decode(&json!({"data":data})).is_err());
    }
    for name in ["bad\\nname".replace("\\n", "\n"), "x".repeat(2049)] {
        assert!(get().decode(&json!({"data":{"name":name}})).is_err());
    }
}
