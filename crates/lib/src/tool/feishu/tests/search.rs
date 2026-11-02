use super::*;

fn search() -> Search {
    Search {
        query: String::new(),
        start: 1793323772,
        end: 1793582972,
        size: 2,
        cursor: Some("previous".into()),
    }
}

fn record() -> Value {
    let item = json!({"meta_data":{"message_id":"om_fixture"},"unprojected":"withheld"});
    json!({"data":{"items":[item],"has_more":true,"page_token":"next"}})
}

#[test]
fn request() {
    let call = search().call().unwrap();
    assert_eq!(call.method, "POST");
    assert_eq!(
        call.path,
        "im/v1/messages/search?page_size=2&page_token=previous"
    );
    assert_eq!(
        call.body.unwrap(),
        json!({"query":"","filter":{"time_range":{
            "start_time":"2026-10-30T01:29:32+00:00","end_time":"2026-11-02T01:29:32+00:00"
        }}})
    );
    let mut request = search();
    request.query = "待办 + &事项".into();
    request.cursor = Some("x+y&z".into());
    let call = request.call().unwrap();
    assert!(call.path.ends_with("page_token=x%2By%26z"));
    assert_eq!(call.body.unwrap()["query"], request.query);
}

#[test]
fn pages() {
    let search = search();
    let result = search.decode(&record()).unwrap();
    assert_eq!(result.items, ["om_fixture"]);
    assert_eq!(result.cursor.as_deref(), Some("next"));
    assert!(result.more);
    for cursor in [json!(null), json!(""), json!("previous"), json!(10)] {
        let mut value = record();
        value["data"]["page_token"] = cursor;
        assert!(search.decode(&value).is_err());
    }
    for data in [
        json!({"items":[],"has_more":true,"page_token":"next"}),
        json!({"items":[]}),
        json!({"has_more":false}),
        json!({"items":null,"has_more":false}),
        json!({"items":[{"meta_data":{"message_id":"om_x"}},{"meta_data":{"message_id":"om_x"}}],"has_more":false}),
        json!({"items":[{"meta_data":{"message_id":"om_x"}},{"meta_data":{"message_id":"om_y"}},{"meta_data":{"message_id":"om_z"}}],"has_more":false}),
        json!({"items":[{"meta_data":{"message_id":"om_"}}],"has_more":false}),
        json!({"items":[{"message_id":"om_missing_meta"}],"has_more":false}),
    ] {
        assert!(search.decode(&json!({"data":data})).is_err());
    }
    let empty = search
        .decode(&json!({"data":{"items":[],"has_more":false}}))
        .unwrap();
    assert!(empty.items.is_empty());
    assert!(!empty.more);
}

#[test]
fn inputs() {
    for (start, end, size) in [
        (-1, 1, 1),
        (1, 1, 1),
        (2, 1, 1),
        (0, i64::MAX, 1),
        (0, 1, 0),
        (0, 1, 51),
    ] {
        let mut request = search();
        request.start = start;
        request.end = end;
        request.size = size;
        assert!(request.validate().is_err());
    }
    for cursor in ["".into(), "bad\n".into(), "x".repeat(2049)] {
        let mut request = search();
        request.cursor = Some(cursor);
        assert!(request.validate().is_err());
    }
    for query in ["x".repeat(4097), "bad\n".into()] {
        let mut request = search();
        request.query = query;
        assert!(request.validate().is_err());
    }
}
