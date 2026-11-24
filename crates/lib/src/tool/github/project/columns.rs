use super::{Call, Cause, Client, Error, Receipt, Window, node, variables};
use serde::{Deserialize, Serialize};
use url::{Url, form_urlencoded::Serializer};

#[derive(Debug, Deserialize, Serialize)]
pub struct Column {
    pub id: u64,
    #[serde(rename = "node_id")]
    pub node: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct Columns {
    pub rows: Vec<Column>,
    pub next: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Locator {
    owner: String,
    number: u32,
    organization: u64,
}

impl Locator {
    pub fn new(owner: &str, number: u32, organization: u64) -> Result<Self, Error> {
        variables(owner, number)?;
        if organization == 0 {
            return Err(Error::local(Cause::Input));
        }
        Ok(Self {
            owner: owner.into(),
            number,
            organization,
        })
    }

    pub(super) fn route(&self) -> String {
        format!("/orgs/{}/projectsV2/{}", self.owner, self.number)
    }

    pub(super) fn address(&self, url: &str, suffix: &str) -> bool {
        let named = format!("https://api.github.com{}{suffix}", self.route());
        let numbered = format!(
            "https://api.github.com/organizations/{}/projectsV2/{}{suffix}",
            self.organization, self.number
        );
        url.eq_ignore_ascii_case(&named) || url.eq_ignore_ascii_case(&numbered)
    }
}

impl Client {
    pub fn columns(&self, location: &Locator, window: &Window) -> Result<Receipt<Columns>, Error> {
        let route = format!("{}/fields", location.route());
        let mut query = Serializer::new(String::new());
        query.append_pair("per_page", &window.size.to_string());
        if let Some(after) = &window.after {
            query.append_pair("after", after);
        }
        let path = format!("{route}?{}", query.finish());
        self.wire.inspect(
            &self.bearer,
            Call::Recent(&path, None),
            |value, response| {
                let rows: Vec<Column> = serde_json::from_value(value).ok()?;
                if rows.len() > usize::from(window.size) || !rows.iter().all(Column::valid) {
                    return None;
                }
                let link = response.headers.get("link").map(|header| header.as_str());
                let next = continuation(link, location, window)?;
                if rows.is_empty() && next.is_some() {
                    return None;
                }
                Some(Columns { rows, next })
            },
        )
    }
}

impl Column {
    fn valid(&self) -> bool {
        self.id != 0 && node(&self.node) && !self.name.trim().is_empty()
    }
}

fn continuation(
    header: Option<&str>,
    location: &Locator,
    window: &Window,
) -> Option<Option<String>> {
    let Some(header) = header else {
        return Some(None);
    };
    let mut next = None;
    for pair in header.split(',') {
        let (url, relation) = pair.trim().split_once('>')?;
        let url = url.strip_prefix('<')?;
        if relation
            .split(';')
            .any(|part| part.trim() == "rel=\"next\"")
        {
            if next.is_some() {
                return None;
            }
            next = Some(cursor(url, location, window)?);
        }
    }
    Some(next)
}

fn cursor(url: &str, location: &Locator, window: &Window) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let (address, _) = url.split_once('?')?;
    if !location.address(address, "/fields") || parsed.fragment().is_some() {
        return None;
    }
    let mut after = None;
    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "after" if after.is_none() => after = Some(value.into_owned()),
            "per_page" if value.parse::<u8>().ok() == Some(window.size) => (),
            _ => return None,
        }
    }
    let after = after?;
    Window::new(window.size, Some(&after)).ok()?;
    (window.after.as_deref() != Some(&after)).then_some(after)
}
