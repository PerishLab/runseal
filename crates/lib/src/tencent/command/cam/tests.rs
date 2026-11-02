use super::*;

#[test]
fn projection() {
    use super::model;
    use crate::tencent::api::project;
    let value = json!({
        "RequestId":"fixture-id", "PolicyId":99,
        "AccessKey":{"AccessKeyId":"public-id", "SecretAccessKey":"PRIVATE_MARKER", "PolicyId":33},
        "SecretKey":"PRIVATE_MARKER"
    });
    assert_eq!(
        project::<model::Issued>(&value).unwrap(),
        json!({
            "RequestId":"fixture-id", "AccessKey":{"AccessKeyId":"public-id"}
        })
    );
    assert_eq!(
        project::<model::Ack>(&value).unwrap(),
        json!({"RequestId":"fixture-id"})
    );
    assert_eq!(
        project::<model::Created>(&value).unwrap(),
        json!({"RequestId":"fixture-id", "PolicyId":99})
    );
    for value in [
        json!({"RequestId":"fixture-id"}),
        json!({"RequestId":"fixture-id", "AccessKeys":[{}]}),
        json!({"RequestId":"fixture-id", "AccessKeys":[{"AccessKeyId":""}]}),
        json!({"RequestId":"fixture-id", "AccessKeys":[{"AccessKeyId":{"SecretKey":"PRIVATE_MARKER"}}]}),
        json!({"RequestId":"fixture-id", "AccessKeys":vec![json!({"AccessKeyId":"public-id"});201]}),
        json!({"RequestId":"fixture-id", "AccessKeys":[{"AccessKeyId":"public-id", "Description":"x".repeat(128*1024+1)}]}),
    ] {
        let error = project::<model::Keys>(&value).unwrap_err();
        let fault = error.downcast_ref::<crate::tencent::Failure>().unwrap();
        assert_eq!(fault.request.as_deref(), Some("fixture-id"));
        assert!(!format!("{error:?}").contains("PRIVATE_MARKER"));
    }
    for keys in [
        json!(null),
        json!([]),
        json!(vec![json!({"AccessKeyId":"public-id"}); 200]),
    ] {
        let output =
            project::<model::Keys>(&json!({"RequestId":"fixture-id", "AccessKeys":keys})).unwrap();
        assert_eq!(output["AccessKeys"], keys);
    }
    for id in [
        json!(""),
        json!("PRIVATE_MARKER\n"),
        json!(7),
        json!("x".repeat(129)),
    ] {
        let error = project::<model::Ack>(&json!({"RequestId":id})).unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<crate::tencent::Failure>()
                .unwrap()
                .request,
            None
        );
        assert!(!format!("{error:?}").contains("PRIVATE_MARKER"));
    }
    assert!(
        project::<model::User>(&json!({"RequestId":"fixture-id", "Uin":"not-number"})).is_err()
    );
    assert!(
        project::<model::Policies>(
            &json!({"RequestId":"fixture-id", "List":[{"PolicyId":1,"Type":"PRIVATE_MARKER"}]})
        )
        .is_err()
    );
}

#[cfg(unix)]
#[test]
fn delivery() {
    use crate::tencent::{deliver, secret::Reserved};
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir_in(std::fs::canonicalize("/tmp").unwrap()).unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.path().join("issued.json");
    let body = json!({"RequestId":"fixture-id", "AccessKey":{"AccessKeyId":"public-id", "SecretAccessKey":"PRIVATE_MARKER", "PolicyId":99}});
    let result = crate::tencent::api::project::<super::model::Issued>(&body).unwrap();
    let receipt = deliver(Reserved::open(&path).unwrap(), &body, 123, result.clone()).unwrap();
    assert_eq!(receipt["result"], json!({"AccessKeyId":"public-id"}));
    assert_eq!(receipt["requestId"], "fixture-id");
    assert!(!format!("{receipt:?}").contains("PRIVATE_MARKER"));
    let bytes = crate::custody::Private::current()
        .read(&path, 16384)
        .unwrap();
    let state: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(state["version"], 1);
    assert_eq!(state["credential"]["key"], "PRIVATE_MARKER");
    let refused = root.path().join("incomplete.json");
    let error = deliver(
        Reserved::open(&refused).unwrap(),
        &json!({"AccessKey":{"AccessKeyId":"public-id"}}),
        123,
        result,
    )
    .unwrap_err();
    assert!(error.to_string().contains("may have completed"));
    assert!(error.to_string().contains("fixture-id"));
    assert!(!format!("{error:?}").contains("PRIVATE_MARKER"));
    assert!(!refused.exists());
}
