use super::{Call, Cause, Client, Error, Page, Receipt, Target, Window, graph, node, page, valid};
use crate::github::{Identity, Outcome};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const SCHEMA: &str = "query RunsealIssueFields($owner:String!,$size:Int!,$after:String) { organization(login:$owner) { id login list:issueFields(first:$size,after:$after) { total:totalCount rows:nodes {
    kind:__typename
    ... on IssueFieldSingleSelect { id name datatype:dataType visibility options { id name } }
    ... on IssueFieldText { id }
    ... on IssueFieldNumber { id }
    ... on IssueFieldDate { id }
    ... on IssueFieldMultiSelect { id }
} page:pageInfo { more:hasNextPage cursor:endCursor } } } }";
const VALUES: &str = "query RunsealIssueValues($owner:String!,$name:String!,$number:Int!,$size:Int!,$after:String) { repository(owner:$owner,name:$name) { nameWithOwner issue(number:$number) { id number list:issueFieldValues(first:$size,after:$after) { total:totalCount rows:nodes {
    kind:__typename
    ... on IssueFieldSingleSelectValue { id option:optionId name field { ... on Node { id } } }
    ... on IssueFieldTextValue { id text:value field { ... on Node { id } } }
    ... on IssueFieldNumberValue { id number:value field { ... on Node { id } } }
    ... on IssueFieldDateValue { id date:value field { ... on Node { id } } }
    ... on IssueFieldMultiSelectValue { id multiple:value field { ... on Node { id } } }
} page:pageInfo { more:hasNextPage cursor:endCursor } } } } }";
const WRITE: &str = "mutation RunsealIssueFieldValue($input:SetIssueFieldValueInput!) { change:setIssueFieldValue(input:$input) { issue { id } } }";

#[derive(Debug, Deserialize, Serialize)]
pub struct Definition {
    pub kind: String,
    pub id: Option<String>,
    pub name: Option<String>,
    pub datatype: Option<String>,
    pub visibility: Option<String>,
    pub options: Option<Vec<Choice>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Choice {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Observation {
    pub kind: String,
    pub id: Option<String>,
    pub field: Option<Identity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub option: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple: Option<Multiple>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Multiple {
    Text(String),
    Options(Vec<String>),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    pub issue: String,
    pub field: String,
    #[serde(deserialize_with = "choice")]
    pub option: Option<String>,
    pub rationale: String,
}

impl Client {
    pub fn definitions(
        &self,
        owner: &str,
        window: &Window,
    ) -> Result<Receipt<Page<Definition>>, Error> {
        if !valid(owner, false) {
            return Err(Error::local(Cause::Input));
        }
        let input = json!({"owner":owner,"size":window.size,"after":window.after});
        self.wire
            .send(&self.bearer, Call::Query(graph(SCHEMA, input)), |value| {
                let scope = value.pointer("/data/organization")?;
                if !scope.get("login")?.as_str()?.eq_ignore_ascii_case(owner) {
                    return None;
                }
                let found: Page<Definition> = page::page(scope, window)?;
                found.rows.iter().all(Definition::valid).then_some(found)
            })
    }

    pub fn observations(
        &self,
        target: &Target,
        window: &Window,
    ) -> Result<Receipt<Page<Observation>>, Error> {
        let mut input = target.variables();
        input["size"] = json!(window.size);
        input["after"] = json!(window.after);
        self.wire
            .send(&self.bearer, Call::Query(graph(VALUES, input)), |value| {
                target.issue(&value)?;
                let found: Page<Observation> =
                    page::page(value.pointer("/data/repository/issue")?, window)?;
                found.rows.iter().all(Observation::valid).then_some(found)
            })
    }

    pub fn select(&self, update: &Update) -> Result<Receipt<Identity>, Error> {
        let input = update.input().ok_or_else(|| Error::local(Cause::Input))?;
        let mut receipt =
            self.wire
                .send(&self.bearer, Call::Mutation(graph(WRITE, input)), |value| {
                    let found: Identity = super::field(&value, "/data/change/issue")?;
                    (found.id == update.issue).then_some(found)
                })?;
        receipt.outcome = Outcome::Accepted;
        Ok(receipt)
    }
}

impl Definition {
    fn valid(&self) -> bool {
        if self.kind != "IssueFieldSingleSelect" {
            return !self.kind.is_empty() && self.id.as_deref().is_none_or(node);
        }
        let required = [
            self.name.as_deref(),
            self.datatype.as_deref(),
            self.visibility.as_deref(),
        ];
        self.id.as_deref().is_some_and(node)
            && required
                .into_iter()
                .all(|value| value.is_some_and(|text| !text.is_empty()))
            && self
                .options
                .as_ref()
                .is_some_and(|options| options.iter().all(|option| node(&option.id)))
    }
}

impl Observation {
    fn valid(&self) -> bool {
        if self.kind == "IssueFieldSingleSelectValue" {
            return self.id.as_deref().is_some_and(node)
                && self.option.as_deref().is_some_and(node)
                && self.field.as_ref().is_some_and(|field| node(&field.id));
        }
        !self.kind.is_empty() && self.id.as_deref().is_none_or(node)
    }
}

impl Update {
    fn input(&self) -> Option<Value> {
        if !node(&self.issue) || !node(&self.field) || self.rationale.trim().is_empty() {
            return None;
        }
        let mut value = json!({"fieldId":self.field,"rationale":self.rationale});
        match &self.option {
            Some(option) if node(option) => value["singleSelectOptionId"] = json!(option),
            Some(_) => return None,
            None => value["delete"] = json!(true),
        }
        Some(json!({"input":{"issueId":self.issue,"issueFields":[value]}}))
    }
}

fn choice<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Option<String>, D::Error> {
    Option::deserialize(input)
}
