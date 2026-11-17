use super::{Data, Request, Slot, provider, wire};
use crate::alibaba::Outcome;
use serde_json::json;

const ACCOUNT: &str = "1234567890123456";

fn input() -> provider::Input {
    provider::Input {
        name: "ensign".into(),
        issuer: "https://identity.example/api".into(),
        clients: vec!["hardrig-aliyun".into()],
        fingerprints: vec!["a1b2c3".into()],
        hours: 6,
        description: None,
    }
}

#[test]
fn versions() {
    let ims = Request::Register(input()).plan().unwrap();
    assert_eq!(
        (ims.host, ims.version, ims.action),
        ("ims.aliyuncs.com", "2019-08-15", "CreateOIDCProvider")
    );
    let ram = Request::Attach {
        role: "operator".into(),
        policy: "read-only".into(),
    }
    .plan()
    .unwrap();
    assert_eq!(
        (ram.host, ram.version, ram.action),
        ("ram.aliyuncs.com", "2015-05-01", "AttachPolicyToRole")
    );
    assert_eq!(ram.fields["PolicyType"], "Custom");
    assert_eq!(
        wire::query(&ram),
        "PolicyName=read-only&PolicyType=Custom&RoleName=operator"
    );
}

#[test]
fn canonical() {
    let mut data = input();
    data.description = Some("space + / ~ 中文".into());
    let query = wire::query(&Request::Register(data).plan().unwrap());
    assert!(query.contains("Description=space%20%2B%20%2F%20~%20%E4%B8%AD%E6%96%87"));
    assert!(query.contains("IssuerUrl=https%3A%2F%2Fidentity.example%2Fapi"));
    assert!(!query.contains('+'));
}

#[test]
fn issuer() {
    for value in [
        "http://identity.example",
        "https://user:pass@identity.example",
        "https://identity.example?x=1",
        "https://identity.example/#x",
    ] {
        let mut data = input();
        data.issuer = value.into();
        assert!(Request::Register(data).plan().is_err());
    }
    let mut data = input();
    data.clients.push(data.clients[0].clone());
    assert!(Request::Register(data).plan().is_err());
}

#[test]
fn projection() {
    let request = Request::Provider {
        name: "ensign".into(),
    };
    let mut body = json!({"RequestId":"receipt-1", "OIDCProvider": {
        "OIDCProviderName":"ensign", "Arn":format!("acs:ram::{ACCOUNT}:oidc-provider/ensign"),
        "IssuerUrl":"https://identity.example/api", "ClientIds":null, "Fingerprints":"", "IssuanceLimitTime":0
    }});
    let (_, data, known) =
        wire::response(200, &serde_json::to_vec(&body).unwrap(), &request, ACCOUNT).unwrap();
    assert!(known);
    let Data::Provider(data) = data else {
        panic!("provider projection");
    };
    assert!(matches!(data.clients, Slot::Null));
    assert!(matches!(data.hours, Slot::Value(0)));
    assert!(matches!(data.fingerprints, Slot::Value(ref value) if value.is_empty()));
    assert!(matches!(data.description, Slot::Missing));
    body["NewField"] = json!("not projected");
    assert!(
        !wire::response(200, &serde_json::to_vec(&body).unwrap(), &request, ACCOUNT)
            .unwrap()
            .2
    );
    body["OIDCProvider"]["Arn"] = json!("acs:ram::999:oidc-provider/ensign");
    assert!(wire::response(200, &serde_json::to_vec(&body).unwrap(), &request, ACCOUNT).is_err());
}

#[test]
fn faults() {
    let request = Request::Attach {
        role: "operator".into(),
        policy: "read-only".into(),
    };
    let body = br#"{"RequestId":"receipt-1","Code":"Forbidden.RAM","Message":"PRIVATE_MARKER"}"#;
    let rejected = wire::response(403, body, &request, ACCOUNT).unwrap_err();
    assert_eq!(rejected.outcome, Outcome::Rejected);
    assert!(!rejected.to_string().contains("PRIVATE_MARKER"));
    assert_eq!(
        wire::response(503, body, &request, ACCOUNT)
            .unwrap_err()
            .outcome,
        Outcome::Unknown
    );
    assert_eq!(
        wire::response(200, body, &request, ACCOUNT)
            .unwrap_err()
            .outcome,
        Outcome::Unknown
    );
    assert_eq!(
        wire::response(200, b"not-json", &request, ACCOUNT)
            .unwrap_err()
            .outcome,
        Outcome::Unknown
    );
    let (_, data, complete) =
        wire::response(200, br#"{"RequestId":"receipt-1"}"#, &request, ACCOUNT).unwrap();
    assert!(matches!(data, Data::Ack));
    assert!(complete);
    assert!(wire::response(200, b"{}", &request, ACCOUNT).is_err());
}

#[test]
fn role() {
    let request = Request::Role {
        name: "operator".into(),
    };
    let mut body = json!({"RequestId":"receipt-1","Role": {
        "RoleName":"operator","RoleId":"9012345", "Arn":format!("acs:ram::{ACCOUNT}:role/operator"),
        "IsServiceLinkedRole":false,"LatestDeletionTask":{},"MaxSessionDuration":3600
    }});
    let (_, data, known) =
        wire::response(200, &serde_json::to_vec(&body).unwrap(), &request, ACCOUNT).unwrap();
    assert!(known);
    let Data::Role(data) = data else {
        panic!("role projection");
    };
    assert_eq!(data.id, "9012345");
    assert!(matches!(data.linked, Slot::Value(false)));
    assert!(matches!(
        data.deletion,
        Slot::Value(super::role::Deletion {
            id: Slot::Missing,
            ..
        })
    ));
    body["Role"]["RoleName"] = json!("other");
    assert!(wire::response(200, &serde_json::to_vec(&body).unwrap(), &request, ACCOUNT).is_err());
}

#[test]
fn policy() {
    let request = Request::Policy {
        name: "read-only".into(),
    };
    let mut body = json!({"RequestId":"receipt-1","Policy": {
        "PolicyName":"read-only", "PolicyType":"Custom", "DefaultVersion":"v1", "AttachmentCount":0,
        "PolicyDocument":"N/A"
    }, "DefaultPolicyVersion":{"VersionId":"v1","IsDefaultVersion":true,"PolicyDocument":"{}"}});
    let (_, data, known) =
        wire::response(200, &serde_json::to_vec(&body).unwrap(), &request, ACCOUNT).unwrap();
    assert!(known);
    let Data::Policy(data) = data else {
        panic!("policy projection");
    };
    assert!(matches!(data.attachments, Slot::Value(0)));
    assert!(
        matches!(data.document, Slot::Value(ref data) if matches!(&data.document, Slot::Value(value) if value == "{}"))
    );
    body["DefaultPolicyVersion"]["VersionId"] = json!("v2");
    assert!(wire::response(200, &serde_json::to_vec(&body).unwrap(), &request, ACCOUNT).is_err());
}

#[test]
fn list() {
    let request = Request::Policies {
        role: "operator".into(),
    };
    let mut body = json!({"RequestId":"receipt-1","Policies":{"Policy":[]}});
    let (_, data, known) =
        wire::response(200, &serde_json::to_vec(&body).unwrap(), &request, ACCOUNT).unwrap();
    assert!(known);
    assert!(matches!(data, Data::Policies(ref entries) if entries.is_empty()));
    body["Policies"]["Policy"] = json!(null);
    assert!(wire::response(200, &serde_json::to_vec(&body).unwrap(), &request, ACCOUNT).is_err());
    body["Policies"]["Policy"] =
        json!([{"PolicyName":"x","PolicyType":"System"},{"PolicyName":"x","PolicyType":"System"}]);
    assert!(wire::response(200, &serde_json::to_vec(&body).unwrap(), &request, ACCOUNT).is_err());
}

#[test]
fn creation() {
    let request = Request::Create(super::role::Input {
        name: "operator".into(),
        trust: json!({"Version":"1","Statement":[]}),
        lifetime: 3600,
        description: None,
    });
    let plan = request.plan().unwrap();
    assert_eq!(plan.fields["AllowConsoleLogin"], "false");
    assert_eq!(plan.fields["MaxSessionDuration"], "3600");
    assert!(request.mutation());
    let request = Request::Define(super::policy::Input {
        name: "read-only".into(),
        document: json!([]),
        description: None,
    });
    assert!(request.plan().is_err());
    for version in ["v0", "v+1", "v-1", "v01", "v", "v18446744073709551616"] {
        let request = Request::Version {
            name: "read-only".into(),
            version: version.into(),
        };
        assert!(request.plan().is_err());
    }
}
