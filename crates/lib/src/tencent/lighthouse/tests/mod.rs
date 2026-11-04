use super::*;
use serde_json::{Value, json};

mod custody;
mod firewall;
mod fixture;
mod instance;
mod key;
mod mutation;
mod protocol;

fn request(operation: Operation) -> Request {
    Request {
        region: "na-ashburn".into(),
        operation,
    }
}

fn instance() -> Request {
    request(Operation::Instance {
        id: "lhins-fixture".into(),
    })
}

fn key() -> Request {
    request(Operation::Key {
        id: "lhkp-fixture".into(),
    })
}

fn keys() -> Request {
    request(Operation::Keys(Window {
        offset: 0,
        limit: 20,
    }))
}

fn identity() -> crate::tencent::Identity {
    crate::tencent::Identity {
        account: "123".into(),
        principal: "456".into(),
        kind: "CAMUser".into(),
        stamp: "fixture".into(),
        observed: 1,
        request: "identity-id".into(),
    }
}
