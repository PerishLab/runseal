use super::{Cause, Error, field, node};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub(super) size: u8,
    pub(super) after: Option<String>,
}

impl Window {
    pub fn new(size: u8, after: Option<&str>) -> Result<Self, Error> {
        if !(1..=100).contains(&size) || after.is_some_and(|value| !cursor(value)) {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            size,
            after: after.map(str::to_owned),
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Page<T> {
    pub target: String,
    pub total: u32,
    pub rows: Vec<T>,
    pub next: Option<String>,
}

#[derive(Deserialize)]
struct Info {
    more: bool,
    cursor: Option<String>,
}

pub(super) fn page<T: DeserializeOwned>(scope: &Value, window: &Window) -> Option<Page<T>> {
    let target = scope.get("id")?.as_str()?;
    if !node(target) {
        return None;
    }
    let group = scope.get("list")?;
    let rows: Vec<T> = field(group, "/rows")?;
    let total: u32 = field(group, "/total")?;
    let info: Info = field(group, "/page")?;
    let count = rows.len();
    if count > usize::from(window.size) || count as u64 > u64::from(total) {
        return None;
    }
    let next = if info.more {
        Some(advance(info.cursor?, window, count)?)
    } else {
        None
    };
    Some(Page {
        target: target.into(),
        total,
        rows,
        next,
    })
}

fn advance(value: String, window: &Window, count: usize) -> Option<String> {
    (count > 0 && cursor(&value) && window.after.as_deref() != Some(value.as_str()))
        .then_some(value)
}

fn cursor(value: &str) -> bool {
    !value.is_empty() && value.len() <= 4096 && value.bytes().all(|byte| byte.is_ascii_graphic())
}
