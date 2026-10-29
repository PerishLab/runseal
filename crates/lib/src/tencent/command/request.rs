use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::tencent) enum Mode {
    Read,
    Write,
}

pub(in crate::tencent) struct Request {
    pub action: &'static str,
    pub body: Value,
    pub mode: Mode,
}

impl Request {
    pub(super) fn read(action: &'static str, body: Value) -> Self {
        Self {
            action,
            body,
            mode: Mode::Read,
        }
    }

    pub(super) fn write(action: &'static str, body: Value) -> Self {
        Self {
            action,
            body,
            mode: Mode::Write,
        }
    }
}
