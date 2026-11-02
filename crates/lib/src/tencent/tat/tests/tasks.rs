use super::{
    execution::{page, query, row},
    *,
};

fn observe(row: Value, output: bool) -> Task {
    let Data::Task {
        value: Some(value), ..
    } = request(Operation::Task(query(output)))
        .decode(&page(row))
        .unwrap()
    else {
        panic!("task");
    };
    *value
}

#[test]
fn result() {
    let value = observe(row(), true);
    assert!(value.complete);
    assert_eq!(value.stage.as_ref().map(Stage::value), Some("SUCCESS"));
    let doc = value.document.unwrap();
    assert_eq!(doc.user.as_deref(), Some("root"));
    assert_eq!(doc.directory.as_deref(), Some("/"));
    assert_eq!(
        doc.digest,
        Some(super::super::script::digest(
            execution::script().content.as_bytes()
        ))
    );
    let result = value.result.unwrap();
    assert_eq!(result.exit, Some(0));
    assert_eq!(result.dropped, Some(0));
    assert_eq!(result.output.as_deref(), Some(b"public-nonce\n".as_slice()));
    let mut value = row();
    value["TaskResult"]["ExitCode"] = json!(-1);
    value["TaskResult"]["Dropped"] = json!(7);
    let result = observe(value, true).result.unwrap();
    assert_eq!(result.exit, Some(-1));
    assert_eq!(result.dropped, Some(7));
}

#[test]
fn hidden() {
    let mut value = row();
    value["TaskResult"]["Output"] = json!("PRIVATE_MARKER");
    let result = observe(value, false);
    assert!(result.result.as_ref().unwrap().output.is_none());
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("PRIVATE_MARKER")
    );
    let mut value = row();
    value["CommandDocument"]["OutputCOSBucketUrl"] = json!("PRIVATE_MARKER");
    value["TaskResult"]["OutputUrl"] = json!("PRIVATE_MARKER");
    let result = observe(value, true);
    assert!(!result.complete);
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("PRIVATE_MARKER")
    );
}

#[test]
fn upload() {
    for field in ["OutputUrl", "OutputUploadCOSErrorInfo"] {
        for empty in [Value::Null, json!("")] {
            let mut value = row();
            value["TaskResult"][field] = empty;
            assert!(observe(value, true).complete);
        }
        let mut value = row();
        value["TaskResult"].as_object_mut().unwrap().remove(field);
        assert!(observe(value, true).complete);
        for withheld in [json!("PRIVATE_MARKER"), json!(true), json!({})] {
            let mut value = row();
            value["TaskResult"][field] = withheld;
            let result = observe(value, true);
            assert!(!result.complete);
            assert!(!result.result.as_ref().unwrap().complete);
            assert!(
                !serde_json::to_string(&result)
                    .unwrap()
                    .contains("PRIVATE_MARKER")
            );
        }
    }
    let mut value = row();
    value["TaskResult"]["Foreign"] = json!("PRIVATE_MARKER");
    assert!(!observe(value, true).complete);
}

#[test]
fn states() {
    for state in [
        "PENDING",
        "DELIVERING",
        "DELIVER_DELAYED",
        "DELIVER_FAILED",
        "START_FAILED",
        "RUNNING",
        "SUCCESS",
        "FAILED",
        "TIMEOUT",
        "TASK_TIMEOUT",
        "CANCELLING",
        "CANCELLED",
        "TERMINATED",
    ] {
        let mut value = row();
        value["TaskStatus"] = json!(state);
        let result = observe(value, false);
        assert_eq!(result.stage.as_ref().map(Stage::value), Some(state));
    }
    let mut value = row();
    value["TaskStatus"] = json!("PRIVATE_MARKER");
    value["InvocationSource"] = json!("PRIVATE_MARKER");
    value["CommandDocument"]["CommandType"] = json!("PRIVATE_MARKER");
    value["Foreign"] = json!("PRIVATE_MARKER");
    let result = observe(value, true);
    assert!(!result.complete);
    assert_eq!(result.stage.unwrap().value(), "UNKNOWN");
}

#[test]
fn nullable() {
    for field in [
        "TaskStatus",
        "TaskResult",
        "CommandDocument",
        "CreatedTime",
        "UpdatedTime",
        "StartTime",
        "EndTime",
        "InvocationSource",
    ] {
        for null in [true, false] {
            let mut value = row();
            if null {
                value[field] = Value::Null;
            } else {
                value.as_object_mut().unwrap().remove(field);
            }
            observe(value, false);
        }
    }
    for field in [
        "ExitCode",
        "Dropped",
        "Output",
        "ExecStartTime",
        "ExecEndTime",
    ] {
        let mut value = row();
        value["TaskResult"][field] = Value::Null;
        let result = observe(value, true).result.unwrap();
        match field {
            "ExitCode" => assert!(result.exit.is_none()),
            "Dropped" => assert!(result.dropped.is_none()),
            "Output" => assert!(result.output.is_none()),
            "ExecStartTime" => assert!(result.interval.start.is_none()),
            _ => assert!(result.interval.end.is_none()),
        }
    }
    let mut value = row();
    value["TaskResult"]["Output"] = json!("");
    assert_eq!(observe(value, true).result.unwrap().output, Some(vec![]));
}

#[test]
fn conflicts() {
    for (field, value) in [
        ("InvocationId", "inv-foreign"),
        ("CommandId", "cmd-foreign"),
        ("InstanceId", "lhins-foreign"),
        ("InvocationTaskId", "inv-wrong"),
    ] {
        let mut body = row();
        body[field] = json!(value);
        assert!(
            request(Operation::Task(query(true)))
                .decode(&page(body))
                .is_err()
        );
    }
    let mut query = query(true);
    query.id = Some("invt-foreign".into());
    assert!(
        request(Operation::Task(query))
            .decode(&page(row()))
            .is_err()
    );
    for body in [
        json!({"InvocationTaskSet":[row(),row()],"TotalCount":2}),
        json!({"InvocationTaskSet":[],"TotalCount":1}),
        json!({"InvocationTaskSet":null,"TotalCount":0}),
        json!({"InvocationTaskSet":[]}),
    ] {
        assert!(
            request(Operation::Task(execution::query(true)))
                .decode(&body)
                .is_err()
        );
    }
    let result = request(Operation::Task(execution::query(false)))
        .decode(&json!({"InvocationTaskSet":[],"TotalCount":0}))
        .unwrap();
    assert!(matches!(result, Data::Task { value: None, .. }));
}

#[test]
fn malformed() {
    for (field, value) in [
        ("ExitCode", json!(1.5)),
        ("Dropped", json!(-1)),
        ("Output", json!("PRIVATE_MARKER")),
        ("Output", json!("eA==".repeat(9000))),
        ("ExecStartTime", json!("not-a-time")),
    ] {
        let mut body = row();
        body["TaskResult"][field] = value;
        let failure = request(Operation::Task(query(true)))
            .decode(&page(body))
            .unwrap_err();
        assert!(!format!("{failure:?} {failure}").contains("PRIVATE_MARKER"));
    }
    let mut body = row();
    body["CommandDocument"]["Content"] = json!("invalid");
    assert!(
        request(Operation::Task(query(false)))
            .decode(&page(body))
            .is_err()
    );
}
