use super::*;
use serde_json::json;

fn page() -> Page {
    Page {
        chat: "oc_fixture".into(),
        size: 2,
        cursor: Some("previous".into()),
        start: None,
        end: None,
        ascending: false,
    }
}

fn item() -> serde_json::Value {
    json!({"message_id":"om_fixture","chat_id":"oc_fixture","msg_type":"text",
        "sender":{"id":"ou_user","id_type":"open_id","sender_type":"user"},
        "body":{"content":"{\"text\":\"reply\"}"}})
}

#[test]
fn pagination() {
    let page = page();
    let mut value = json!({"data":{"items":[item()],"has_more":true,"page_token":"next"}});
    assert!(page.decode(&value).unwrap().more);
    for cursor in [json!(null), json!(""), json!("previous"), json!(10)] {
        value["data"]["page_token"] = cursor;
        assert!(page.decode(&value).is_err());
    }
    value["data"] = json!({"items":[],"has_more":false});
    assert!(page.decode(&value).unwrap().items.is_empty());
    value["data"] = json!({"has_more":false});
    assert!(page.decode(&value).is_err());
    value["data"] = json!({"items":[item(),item()],"has_more":false});
    assert!(page.decode(&value).is_err());
    value["data"]["items"] = json!([item()]);
    value["data"]["items"][0]["chat_id"] = json!("oc_other");
    assert!(page.decode(&value).is_err());
}

#[test]
fn sending() {
    let mut send = Send {
        target: Target::Chat("oc_fixture".into()),
        text: "hello".into(),
        uuid: "intent-1".into(),
    };
    let call = send.call().unwrap();
    assert_eq!(call.method, "POST");
    let mut value = json!({"data":item()});
    assert!(Operation::Send(send.clone()).decode(&value).is_ok());
    value["data"]["chat_id"] = json!("oc_other");
    assert!(Operation::Send(send.clone()).decode(&value).is_err());
    send.uuid.clear();
    assert!(Operation::Send(send).validate().is_err());
}
