use super::*;
use crate::feishu::Outcome;

fn message() -> Value {
    json!({"message_id":"om_original","chat_id":"oc_fixture","msg_type":"text",
        "sender":{"id":"ou_owner","id_type":"open_id","sender_type":"user"},
        "body":{"content":"{\"text\":\"question\"}"}})
}

#[test]
fn exact() {
    let get = Operation::Get {
        message: "om_original".into(),
    };
    let held = get.decode(&json!({"data":{"items":[message()]}})).unwrap();
    assert_eq!(held.sender.id, "ou_owner");
    assert_eq!(held.deleted, None);
    for items in [
        json!([]),
        json!([message(), message()]),
        json!([{"message_id":"om_foreign"}]),
    ] {
        assert_eq!(
            get.decode(&json!({"data":{"items":items}}))
                .unwrap_err()
                .outcome,
            Outcome::Unknown
        );
    }
    let mut foreign = message();
    foreign["message_id"] = json!("om_other");
    assert!(get.decode(&json!({"data":{"items":[foreign]}})).is_err());
}

#[test]
fn atomic() {
    let reply = Operation::Reply(Reply {
        message: "om_original".into(),
        text: "第一行\nsecond line".into(),
        uuid: "explicit-uuid".into(),
        thread: true,
    });
    let call = reply.call().unwrap();
    assert_eq!(call.method, "POST");
    assert_eq!(call.path, "im/v1/messages/om_original/reply");
    let body = call.body.unwrap();
    assert_eq!(body["uuid"], "explicit-uuid");
    assert_eq!(body["reply_in_thread"], true);
    assert_eq!(
        serde_json::from_str::<Value>(body["content"].as_str().unwrap()).unwrap()["text"],
        "第一行\nsecond line"
    );
    let mut ack = message();
    ack["message_id"] = json!("om_reply");
    ack["sender"] =
        json!({"id":"cli_fixture","id_type":"app_id","sender_type":"app","open_bot_id":"ou_bot"});
    assert!(reply.decode(&json!({"data":ack})).is_err());
    ack["parent_id"] = json!("om_original");
    assert_eq!(reply.decode(&json!({"data":ack})).unwrap().id, "om_reply");
    ack["parent_id"] = json!("om_other");
    assert!(reply.decode(&json!({"data":ack})).is_err());
}

#[test]
fn sender() {
    let mut record = message();
    record["sender"] = json!({"id":"cli_fixture","id_type":"app_id","sender_type":"app"});
    let held: Message = serde_json::from_value(record).unwrap();
    held.validate().unwrap();
    let mut identity = crate::feishu::Identity {
        app: "cli_fixture".into(),
        bot: "ou_bot".into(),
        observed: 100,
        stamp: "fixture".into(),
    };
    assert!(held.sender.matches(&identity));
    identity.app = "cli_foreign".into();
    assert!(!held.sender.matches(&identity));
}

#[test]
fn input() {
    for id in ["om_", "om_a/../../other", "om_a?x=y", "bad"] {
        assert!(Operation::Get { message: id.into() }.validate().is_err());
    }
    for (text, uuid) in [
        ("", "uuid"),
        (" \n", "uuid"),
        ("text", ""),
        ("text", "has space"),
    ] {
        assert!(
            Operation::Reply(Reply {
                message: "om_original".into(),
                text: text.into(),
                uuid: uuid.into(),
                thread: false
            })
            .validate()
            .is_err()
        );
    }
}
