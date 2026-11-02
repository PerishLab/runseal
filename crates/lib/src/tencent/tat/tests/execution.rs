use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};

pub(super) fn script() -> Script {
    Script {
        instance: "lhins-fixture".into(),
        name: "public-probe".into(),
        user: "root".into(),
        directory: "/".into(),
        timeout: 30,
        content: "printf public-nonce\n".into(),
    }
}

pub(super) fn query(output: bool) -> Query {
    Query {
        target: Target {
            invocation: "inv-fixture".into(),
            command: "cmd-fixture".into(),
            instance: "lhins-fixture".into(),
        },
        id: None,
        output,
    }
}

pub(super) fn row() -> Value {
    json!({"InvocationId":"inv-fixture","CommandId":"cmd-fixture","InvocationTaskId":"invt-fixture",
        "InstanceId":"lhins-fixture","TaskStatus":"SUCCESS","InvocationSource":"USER",
        "CreatedTime":"2026-11-02T04:00:00Z","StartTime":null,"EndTime":null,"UpdatedTime":"2026-11-02T04:00:01Z",
        "CommandDocument":{"Content":STANDARD.encode(script().content),"CommandType":"SHELL","Timeout":30,
            "Username":"root","WorkingDirectory":"/","OutputCOSBucketUrl":"","OutputCOSKeyPrefix":""},
        "TaskResult":{"ExitCode":0,"Output":STANDARD.encode("public-nonce\n"),"Dropped":0,
            "ExecStartTime":"2026-11-02T04:00:00Z","ExecEndTime":"2026-11-02T04:00:01Z","OutputUrl":"",
            "OutputUploadCOSErrorInfo":""},
        "ErrorInfo":"PRIVATE_MARKER","CommandName":"public-probe"})
}

pub(super) fn page(row: Value) -> Value {
    json!({"InvocationTaskSet":[row],"TotalCount":1,"RequestId":"fixture-id"})
}

#[test]
fn shapes() {
    let input = request(Operation::Run(script()));
    let call = input.call().unwrap();
    assert_eq!(call.action, "RunCommand");
    assert_eq!(
        call.body,
        json!({"InstanceIds":["lhins-fixture"],"CommandName":"public-probe","Username":"root",
        "WorkingDirectory":"/","Timeout":30,"CommandType":"SHELL","Content":STANDARD.encode(script().content),
        "SaveCommand":false,"EnableParameter":false})
    );
    assert_eq!(call.request("na-ashburn").region, Some("na-ashburn"));
    let call = request(Operation::Task(query(false))).call().unwrap();
    assert_eq!(call.action, "DescribeInvocationTasks");
    assert_eq!(
        call.body,
        json!({"Offset":0,"Limit":1,"HideOutput":true,"Filters":[
        {"Name":"invocation-id","Values":["inv-fixture"]},{"Name":"instance-id","Values":["lhins-fixture"]}]})
    );
    let mut scoped = query(true);
    scoped.id = Some("invt-fixture".into());
    let call = request(Operation::Task(scoped)).call().unwrap();
    assert_eq!(
        call.body,
        json!({"Offset":0,"Limit":1,"HideOutput":false,"InvocationTaskIds":["invt-fixture"]})
    );
}

#[test]
fn bounds() {
    for (field, value) in [
        ("instance", "ins-foreign"),
        ("name", "bad name"),
        ("user", "root;id"),
        ("directory", "relative"),
        ("content", ""),
    ] {
        let mut candidate = script();
        match field {
            "instance" => candidate.instance = value.into(),
            "name" => candidate.name = value.into(),
            "user" => candidate.user = value.into(),
            "directory" => candidate.directory = value.into(),
            _ => candidate.content = value.into(),
        }
        assert!(candidate.validate().is_err());
    }
    for timeout in [0, 86401] {
        let mut value = script();
        value.timeout = timeout;
        assert!(value.validate().is_err());
    }
    for size in [1, 49152, 49153] {
        let mut value = script();
        value.content = "x".repeat(size);
        assert_eq!(value.validate().is_ok(), size <= 49152);
    }
    let mut value = script();
    value.content = "PRIVATE_MARKER\0".into();
    assert!(value.validate().is_err());
    assert!(!format!("{value:?}").contains("PRIVATE_MARKER"));
    for target in [
        Target {
            invocation: "wrong".into(),
            ..query(false).target
        },
        Target {
            command: "skey-wrong".into(),
            ..query(false).target
        },
        Target {
            instance: "ins-wrong".into(),
            ..query(false).target
        },
    ] {
        assert!(target.validate().is_err());
    }
    let mut value = query(false);
    value.id = Some("inv-wrong".into());
    assert!(request(Operation::Task(value)).validate().is_err());
}

#[test]
fn receipts() {
    let identity = crate::tencent::Identity {
        account: "123".into(),
        principal: "456".into(),
        kind: "CAMUser".into(),
        stamp: "fixture".into(),
        observed: 1,
        request: "identity-id".into(),
    };
    let input = request(Operation::Run(script()));
    let body = json!({"CommandId":"cmd-fixture","InvocationId":"inv-fixture","RequestId":"fixture-id","SecretKey":"PRIVATE_MARKER"});
    let receipt = input.receipt(identity.clone(), &body).unwrap();
    assert_eq!(receipt.outcome, Outcome::Accepted);
    let Data::Dispatched(data) = receipt.data else {
        panic!("dispatch");
    };
    assert_eq!(data.target.instance, "lhins-fixture");
    assert_eq!(
        data.digest,
        "a5ef86bcb4dd8d850d949e18546fb8941b14bac13f6afbc8abdc1826e191b7f3"
    );
    for value in [
        json!({"RequestId":"fixture-id"}),
        json!({"RequestId":"fixture-id","CommandId":"cmd-good","InvocationId":"foreign"}),
    ] {
        let failure = input.receipt(identity.clone(), &value).unwrap_err();
        assert_eq!(failure.request.as_deref(), Some("fixture-id"));
    }
    let receipt = request(Operation::Task(query(true)))
        .receipt(identity, &page(row()))
        .unwrap();
    assert_eq!(receipt.outcome, Outcome::Observed);
    assert!(
        !serde_json::to_string(&receipt)
            .unwrap()
            .contains("PRIVATE_MARKER")
    );
}

#[test]
fn command() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("public.sh");
    std::fs::write(&file, script().content).unwrap();
    let words = [
        "tat",
        "--region",
        "na-ashburn",
        "run",
        "--instance",
        "lhins-fixture",
        "--name",
        "public-probe",
        "--username",
        "root",
        "--directory",
        "/",
        "--timeout",
        "30",
        "--script-file",
        file.to_str().unwrap(),
    ];
    let parsed = crate::tencent::parse(&words.map(str::to_owned)).unwrap();
    let crate::tencent::command::Deed::Tat(deed) = parsed.deed else {
        panic!("TAT");
    };
    let request: Request = deed.convert().unwrap();
    assert_eq!(request.call().unwrap().action, "RunCommand");
    std::fs::write(&file, vec![b'x'; 49153]).unwrap();
    assert!(deed.convert().is_err());
}
