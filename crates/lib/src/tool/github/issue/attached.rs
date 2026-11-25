use super::{Call, Cause, Client, Error, Identity, Receipt, Target, field, graph, node, valid};
use crate::github::{Number, Outcome};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

const BADGE: &str = "query RunsealBadge($owner:String!,$name:String!,$label:String!){repository(owner:$owner,name:$name){id label(name:$label){id name}}}";
const MINT: &str = "mutation RunsealMint($repository:ID!,$name:String!,$color:String!){created:createLabel(input:{repositoryId:$repository,name:$name,color:$color}){label{id name}}}";
const ATTACH: &str = "mutation RunsealAttachLabel($issue:ID!,$labels:[ID!]!){alter:addLabelsToLabelable(input:{labelableId:$issue,labelIds:$labels}){labelable{__typename ... on Issue{id}}}}";
const DETACH: &str = "mutation RunsealDetachLabel($issue:ID!,$labels:[ID!]!){alter:removeLabelsFromLabelable(input:{labelableId:$issue,labelIds:$labels}){labelable{__typename ... on Issue{id}}}}";
const ADMISSION: &str = "query RunsealAdmission($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){id nameWithOwner defaultBranchRef{name} issue(number:$number){id number url state issueType{name} subIssuesSummary{total} labels(first:100){nodes{name} pageInfo{hasNextPage}}} rulesets(first:100,includeParents:true){nodes{enforcement target conditions{refName{include exclude}} bypassActors(first:100){totalCount} rules(first:100){nodes{type} pageInfo{hasNextPage}}} pageInfo{hasNextPage}}}}";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Badge {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Found {
    pub repository: String,
    pub badge: Option<Badge>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Entry {
    #[serde(rename = "issue_field_id")]
    pub field: u64,
    #[serde(rename = "data_type")]
    pub datatype: String,
    #[serde(flatten)]
    pub rest: Map<String, Value>,
}

#[derive(Debug, Serialize)]
pub struct Listing {
    pub page: u32,
    pub size: u8,
    pub rows: Vec<Entry>,
    pub more: bool,
}

impl Client {
    pub fn attached(&self, target: &Target, window: Number) -> Result<Receipt<Listing>, Error> {
        let path = format!(
            "/repos/{}/issues/{}/issue-field-values?per_page={}&page={}",
            target.name(),
            target.number(),
            window.size,
            window.page
        );
        self.wire.send(&self.bearer, Call::Source(&path), |value| {
            let entries = value.as_array()?;
            if entries.len() > usize::from(window.size) {
                return None;
            }
            let mut rows = Vec::with_capacity(entries.len());
            let mut seen = BTreeSet::new();
            for entry in entries {
                let row = Entry::deserialize(entry).ok()?;
                if row.field == 0 || row.datatype.is_empty() || !seen.insert(row.field) {
                    return None;
                }
                rows.push(row);
            }
            let more = rows.len() == usize::from(window.size);
            Some(Listing {
                page: window.page,
                size: window.size,
                rows,
                more,
            })
        })
    }

    pub fn badge(&self, owner: &str, name: &str, label: &str) -> Result<Receipt<Found>, Error> {
        if !valid(owner, false) || !valid(name, true) {
            return Err(Error::local(Cause::Input));
        }
        if label.trim().is_empty() || label.len() > 50 {
            return Err(Error::local(Cause::Input));
        }
        let variables = json!({"owner":owner,"name":name,"label":label});
        self.wire.send(
            &self.bearer,
            Call::Query(graph(BADGE, variables)),
            |value| {
                let repo = value.pointer("/data/repository")?;
                let repository = repo.get("id")?.as_str()?;
                if !node(repository) {
                    return None;
                }
                let raw = repo.get("label")?;
                let badge = if raw.is_null() {
                    None
                } else {
                    let found = Badge::deserialize(raw).ok()?;
                    if !node(&found.id) || found.name != label {
                        return None;
                    }
                    Some(found)
                };
                Some(Found {
                    repository: repository.into(),
                    badge,
                })
            },
        )
    }

    pub fn mint(
        &self,
        repository: &str,
        label: &str,
        color: &str,
    ) -> Result<Receipt<Badge>, Error> {
        if !node(repository) || label.trim().is_empty() || label.len() > 50 {
            return Err(Error::local(Cause::Input));
        }
        if color.len() != 6 || !color.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Error::local(Cause::Input));
        }
        let variables = json!({"repository":repository,"name":label,"color":color});
        let mut receipt = self.wire.send(
            &self.bearer,
            Call::Mutation(graph(MINT, variables)),
            |value| {
                let found: Badge = field(&value, "/data/created/label")?;
                (node(&found.id) && found.name == label).then_some(found)
            },
        )?;
        receipt.outcome = Outcome::Accepted;
        Ok(receipt)
    }

    pub fn relabel(
        &self,
        issue: &str,
        label: &str,
        remove: bool,
    ) -> Result<Receipt<Identity>, Error> {
        if !node(issue) || !node(label) {
            return Err(Error::local(Cause::Input));
        }
        let statement = if remove { DETACH } else { ATTACH };
        let variables = json!({"issue":issue,"labels":[label]});
        let mut receipt = self.wire.send(
            &self.bearer,
            Call::Mutation(graph(statement, variables)),
            |value| {
                let target = value.pointer("/data/alter/labelable")?;
                if target.get("__typename")?.as_str()? != "Issue" {
                    return None;
                }
                let found: Identity = Identity::deserialize(target).ok()?;
                (found.id == issue).then_some(found)
            },
        )?;
        receipt.outcome = Outcome::Accepted;
        Ok(receipt)
    }

    pub fn admission(&self, target: &Target) -> Result<Receipt<Value>, Error> {
        self.wire.send(
            &self.bearer,
            Call::Query(graph(ADMISSION, target.variables())),
            |value| {
                let repo = value.pointer("/data/repository")?;
                let id = repo.get("id")?.as_str()?;
                let name = repo.get("nameWithOwner")?.as_str()?;
                if !node(id) || !name.eq_ignore_ascii_case(&target.name()) {
                    return None;
                }
                let issue = repo.get("issue")?;
                if issue.is_null() {
                    return None;
                }
                Some(repo.clone())
            },
        )
    }
}
