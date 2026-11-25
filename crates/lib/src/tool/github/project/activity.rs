use super::{Call, Cause, Client, Error, Receipt, Window, catalogue, graph, mutation, node};
use crate::github::{Identity, Outcome, field};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const PAGE: &str = "totalCount pageInfo{hasNextPage endCursor}";
const META: &str = "__typename id number title url updatedAt closed public readme shortDescription viewerCanUpdate owner{__typename ... on Organization{id login}}";
const REF: &str = "__typename ... on ProjectV2FieldCommon{id name dataType isIssueField}";
const ORIGIN: &str = "issueField{... on IssueFieldText{id fullDatabaseId name dataType}... on IssueFieldNumber{id fullDatabaseId name dataType}... on IssueFieldDate{id fullDatabaseId name dataType}... on IssueFieldSingleSelect{id fullDatabaseId name dataType options{id name color description}}... on IssueFieldMultiSelect{id fullDatabaseId name dataType}}";
const OBSERVE: &str = "id number title url state body updatedAt issueType{id name} repository{id nameWithOwner} parent{id url} blockedBy(first:100){totalCount pageInfo{hasNextPage endCursor} nodes{id url state}} blocking(first:100){totalCount pageInfo{hasNextPage endCursor} nodes{id url state}} subIssues(first:100){totalCount pageInfo{hasNextPage endCursor} nodes{id url state}} labels(first:100){totalCount pageInfo{hasNextPage endCursor} nodes{id name}} comments(first:100){totalCount pageInfo{hasNextPage endCursor} nodes{id url body updatedAt}}";
const VALUE: &str = "__typename ... on ProjectV2ItemFieldTextValue{id text field{... on ProjectV2FieldCommon{id}}} ... on ProjectV2ItemFieldNumberValue{id number field{... on ProjectV2FieldCommon{id}}} ... on ProjectV2ItemFieldDateValue{id date field{... on ProjectV2FieldCommon{id}}} ... on ProjectV2ItemFieldSingleSelectValue{id optionId name field{... on ProjectV2FieldCommon{id}}} ... on ProjectV2ItemFieldIterationValue{id iterationId title startDate duration field{... on ProjectV2FieldCommon{id}}} ... on ProjectV2ItemFieldMultiSelectValue{id value options{id name} field{... on ProjectV2FieldCommon{id}}} ... on ProjectV2ItemFieldRepositoryValue{field{... on ProjectV2FieldCommon{id}} repository{id nameWithOwner url}} ... on ProjectV2ItemFieldMilestoneValue{field{... on ProjectV2FieldCommon{id}} milestone{id title state url}} ... on ProjectV2ItemFieldLabelValue{field{... on ProjectV2FieldCommon{id}} labels(first:100){totalCount pageInfo{hasNextPage endCursor} nodes{id name}}} ... on ProjectV2ItemFieldUserValue{field{... on ProjectV2FieldCommon{id}} users(first:100){totalCount pageInfo{hasNextPage endCursor} nodes{id login}}} ... on ProjectV2ItemFieldPullRequestValue{field{... on ProjectV2FieldCommon{id}} pullRequests(first:100){totalCount pageInfo{hasNextPage endCursor} nodes{id url state}}} ... on ProjectV2ItemFieldReviewerValue{field{... on ProjectV2FieldCommon{id}} reviewers(first:100){totalCount pageInfo{hasNextPage endCursor} nodes{__typename ... on Node{id}}}} ... on ProjectV2ItemIssueFieldValue{field{... on ProjectV2FieldCommon{id}} issueFieldValue{__typename ... on IssueFieldTextValue{id value field{... on Node{id}}} ... on IssueFieldNumberValue{id value field{... on Node{id}}} ... on IssueFieldDateValue{id value field{... on Node{id}}} ... on IssueFieldSingleSelectValue{id value optionId name field{... on Node{id}}} ... on IssueFieldMultiSelectValue{id value field{... on Node{id}}}}}";
pub(super) const NEXT: &str = "__typename id number title url state updatedAt issueType{id name} repository{id nameWithOwner owner{id}} labels(first:100){totalCount pageInfo{hasNextPage endCursor} nodes{id name}} blockedBy(first:100){totalCount pageInfo{hasNextPage endCursor} nodes{id url state}}";
const MANAGE: &str = "id type isArchived updatedAt project{id} content{__typename ... on Issue{id number url title state updatedAt issueType{id name} repository{id nameWithOwner owner{id}}}}";
const SURVEY: &str = "id type isArchived updatedAt project{id} content{__typename ... on Issue{id number title url state updatedAt issueType{id name} repository{id nameWithOwner owner{id login}}}}";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Section {
    Fields,
    Views,
    Workflows,
    Repositories,
    Updates,
    Items(Detail),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Detail {
    Observe,
    Manage,
    Planning,
    Values,
    Removal,
    Next,
    Survey,
}

#[derive(Debug, Serialize)]
pub struct Slice {
    pub project: Value,
    pub connection: Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Removed {
    pub id: String,
    pub project: Identity,
}

impl Client {
    pub fn board(
        &self,
        project: &str,
        section: Section,
        window: &Window,
    ) -> Result<Receipt<Slice>, Error> {
        if !node(project) {
            return Err(Error::local(Cause::Input));
        }
        let name = section.name();
        let selection = section.selection();
        let statement = format!(
            "query RunsealBoard($id:ID!,$size:Int!,$after:String){{node(id:$id){{... on ProjectV2{{{META} {name}(first:$size,after:$after){{{PAGE} nodes{{{selection}}}}}}}}}}}"
        );
        let variables = json!({"id":project,"size":window.size,"after":window.after});
        self.wire.send(
            &self.bearer,
            Call::Survey(graph(&statement, variables)),
            |value| decode(value, project, name, window),
        )
    }

    pub fn retire(&self, project: &str, workflow: &str) -> Result<Receipt<Removed>, Error> {
        if !node(project) || !node(workflow) {
            return Err(Error::local(Cause::Input));
        }
        let query = mutation(
            "deleteProjectV2Workflow",
            "DeleteProjectV2WorkflowInput",
            "id:deletedWorkflowId project:projectV2 {id}",
        );
        let mut receipt = self.wire.send(
            &self.bearer,
            Call::Mutation(graph(&query, json!({"input":{"workflowId":workflow}}))),
            |value| {
                let found: Removed = field(&value, "/data/change")?;
                (found.id == workflow && found.project.id == project).then_some(found)
            },
        )?;
        receipt.outcome = Outcome::Accepted;
        Ok(receipt)
    }

    pub fn record(&self, project: &str, body: &str) -> Result<Receipt<catalogue::Update>, Error> {
        if !node(project) || body.trim().is_empty() || body.len() > 65_536 {
            return Err(Error::local(Cause::Input));
        }
        let fields = format!("statusUpdate {{ {} }}", catalogue::UPDATE);
        let query = mutation(
            "createProjectV2StatusUpdate",
            "CreateProjectV2StatusUpdateInput",
            &fields,
        );
        let mut receipt = self.wire.send(
            &self.bearer,
            Call::Mutation(graph(
                &query,
                json!({"input":{"projectId":project,"body":body}}),
            )),
            |value| {
                let found: catalogue::Update = field(&value, "/data/change/statusUpdate")?;
                (found.body.as_deref() == Some(body) && found.valid(project)).then_some(found)
            },
        )?;
        receipt.outcome = Outcome::Accepted;
        Ok(receipt)
    }
}

impl Section {
    fn name(self) -> &'static str {
        match self {
            Self::Fields => "fields",
            Self::Views => "views",
            Self::Workflows => "workflows",
            Self::Repositories => "repositories",
            Self::Updates => "statusUpdates",
            Self::Items(_) => "items",
        }
    }

    fn selection(self) -> String {
        match self {
            Self::Fields => format!(
                "__typename ... on ProjectV2FieldCommon{{id name dataType isIssueField updatedAt project{{id}}}} ... on ProjectV2Field{{{ORIGIN}}} ... on ProjectV2SingleSelectField{{{ORIGIN} options{{id name color description}}}} ... on ProjectV2MultiSelectField{{{ORIGIN} multiSelectOptions{{id name color description}}}} ... on ProjectV2IterationField{{configuration{{duration startDay iterations{{id title startDate duration}} completedIterations{{id title startDate duration}}}}}}"
            ),
            Self::Views => format!(
                "id fullDatabaseId number name layout filter updatedAt project{{id}} configuration{{visibleFields(first:100){{pageInfo{{hasNextPage endCursor}} nodes{{{REF}}}}}}} groupByFields(first:100){{{PAGE} nodes{{{REF}}}}} verticalGroupByFields(first:100){{{PAGE} nodes{{{REF}}}}} sortByFields(first:100){{{PAGE} nodes{{direction field{{{REF}}}}}}}"
            ),
            Self::Workflows => "id number name enabled updatedAt project{id}".into(),
            Self::Repositories => "id nameWithOwner url owner{id}".into(),
            Self::Updates => "id body createdAt updatedAt startDate targetDate status".into(),
            Self::Items(Detail::Observe) => format!(
                "id type isArchived updatedAt project{{id}} fieldValues(first:100){{{PAGE} nodes{{{VALUE}}}}} content{{__typename ... on Issue{{{OBSERVE}}} ... on PullRequest{{id number title url state updatedAt repository{{id nameWithOwner}}}} ... on DraftIssue{{id title body updatedAt}}}}"
            ),
            Self::Items(Detail::Manage) => MANAGE.into(),
            Self::Items(Detail::Planning) => planning(false, MANAGE),
            Self::Items(Detail::Values) => planning(true, MANAGE),
            Self::Items(Detail::Removal) => removal(),
            Self::Items(Detail::Next) => planning(
                false,
                &format!(
                    "id type isArchived updatedAt project{{id}} content{{__typename ... on Issue{{{NEXT}}}}}"
                ),
            ),
            Self::Items(Detail::Survey) => planning(true, SURVEY),
        }
    }
}

fn planning(text: bool, item: &str) -> String {
    let kinds = [
        "Date",
        "Label",
        "Milestone",
        "MultiSelect",
        "Number",
        "PullRequest",
        "Repository",
        "Reviewer",
        "Text",
        "User",
    ];
    let refs = kinds
        .iter()
        .map(|kind| {
            let extra = if text && *kind == "Text" { "text " } else { "" };
            format!("... on ProjectV2ItemField{kind}Value{{{extra}field{{... on ProjectV2FieldCommon{{id}}}}}}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "{item} fieldValues(first:100){{{PAGE} nodes{{__typename {refs} ... on ProjectV2ItemIssueFieldValue{{field{{... on ProjectV2FieldCommon{{id}}}}}} ... on ProjectV2ItemFieldSingleSelectValue{{optionId name field{{... on ProjectV2FieldCommon{{id}}}}}} ... on ProjectV2ItemFieldIterationValue{{iterationId title startDate duration field{{... on ProjectV2FieldCommon{{id}}}}}}}}}}"
    )
}

fn removal() -> String {
    let kinds = [
        "Date",
        "Iteration",
        "Label",
        "Milestone",
        "MultiSelect",
        "Number",
        "PullRequest",
        "Repository",
        "Reviewer",
        "SingleSelect",
        "Text",
        "User",
    ];
    let refs = kinds
        .iter()
        .map(|kind| {
            format!(
                "... on ProjectV2ItemField{kind}Value{{field{{... on ProjectV2FieldCommon{{id}}}}}}"
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "{MANAGE} fieldValues(first:100){{{PAGE} nodes{{__typename {refs} ... on ProjectV2ItemIssueFieldValue{{field{{... on ProjectV2FieldCommon{{id}}}}}}}}}}"
    )
}

fn decode(value: Value, expected: &str, name: &str, window: &Window) -> Option<Slice> {
    let mut object = value.pointer("/data/node")?.as_object()?.clone();
    if object.get("id").and_then(Value::as_str) != Some(expected) {
        return None;
    }
    let connection = object.remove(name)?;
    checked(&connection, window)?;
    Some(Slice {
        project: Value::Object(object),
        connection,
    })
}

pub(super) fn checked(group: &Value, window: &Window) -> Option<()> {
    let total = group.get("totalCount")?.as_u64()?;
    let more = group.pointer("/pageInfo/hasNextPage")?.as_bool()?;
    let cursor = group.pointer("/pageInfo/endCursor").and_then(Value::as_str);
    let nodes = group.get("nodes")?.as_array()?;
    if nodes.len() > usize::from(window.size) || nodes.len() as u64 > total {
        return None;
    }
    if more {
        let cursor = cursor?;
        if nodes.is_empty() || cursor.is_empty() || window.after.as_deref() == Some(cursor) {
            return None;
        }
    }
    let mut seen = BTreeSet::new();
    for entry in nodes {
        let id = entry.get("id")?.as_str()?;
        if !node(id) || !seen.insert(id) {
            return None;
        }
    }
    Some(())
}
