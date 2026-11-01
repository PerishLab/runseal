use super::*;

fn record() -> Value {
    json!({"reaction_id":"opaque-record", "operator":{"operator_id":"cli_fixture","operator_type":"app"},"action_time":"1663054162546","reaction_type":{"emoji_type":"Get"}})
}

fn identity() -> Identity {
    Identity {
        app: "cli_fixture".into(),
        bot: "ou_bot".into(),
        user: None,
        observed: 1,
        stamp: "fixture".into(),
    }
}

#[test]
fn creation() {
    let create = Create {
        message: "om_fixture".into(),
        emoji: "Get".into(),
    };
    let call = create.call().unwrap();
    assert_eq!(call.path, "im/v1/messages/om_fixture/reactions");
    assert_eq!(call.body.unwrap()["reaction_type"]["emoji_type"], "Get");
    let mut value = json!({"data":record()});
    assert!(create.decode(&value, &identity()).is_ok());
    value["data"]["reaction_type"]["emoji_type"] = json!("GET");
    assert!(create.decode(&value, &identity()).is_err());
    value["data"] = record();
    value["data"]["operator"]["operator_id"] = json!("cli_foreign");
    assert!(create.decode(&value, &identity()).is_err());
    let mut person = identity();
    person.user = Some("ou_owner".into());
    value["data"]["operator"] = json!({"operator_id":"ou_owner","operator_type":"user"});
    assert!(create.decode(&value, &person).is_ok());
    assert!(create.decode(&value, &identity()).is_err());
    value["data"]["action_time"] = json!("0");
    assert!(create.decode(&value, &person).is_err());
}

#[test]
fn pages() {
    let page = Page {
        message: "om_fixture".into(),
        emoji: Some("Get".into()),
        size: 2,
        cursor: Some("previous".into()),
    };
    assert!(page.call().unwrap().path.contains("user_id_type=open_id"));
    let mut value = json!({"data":{"items":[record()],"has_more":true,"page_token":"next"}});
    assert!(page.decode(&value).unwrap().more);
    for cursor in [json!(null), json!(""), json!("previous"), json!(10)] {
        value["data"]["page_token"] = cursor;
        assert!(page.decode(&value).is_err());
    }
    value["data"] = json!({"items":[],"has_more":false});
    assert!(page.decode(&value).unwrap().items.is_empty());
    value["data"] = json!({"has_more":false});
    assert!(page.decode(&value).is_err());
    value["data"] = json!({"items":[record(),record()],"has_more":false});
    assert!(page.decode(&value).is_err());
    value["data"]["items"] = json!([record()]);
    value["data"]["items"][0]["reaction_type"]["emoji_type"] = json!("SMILE");
    assert!(page.decode(&value).is_err());
}

#[test]
fn inputs() {
    for (message, emoji) in [
        ("om_", "Get"),
        ("om_a?x=y", "Get"),
        ("om_fixture", ""),
        ("om_fixture", "bad&type"),
    ] {
        assert!(
            Create {
                message: message.into(),
                emoji: emoji.into()
            }
            .validate()
            .is_err()
        );
    }
    for size in [0, 51] {
        assert!(
            Page {
                message: "om_fixture".into(),
                emoji: None,
                size,
                cursor: None
            }
            .validate()
            .is_err()
        );
    }
}
