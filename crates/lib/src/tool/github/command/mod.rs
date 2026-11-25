mod acceptance;
mod delivery;
mod display;
use display::execute;
pub(crate) use display::run;
mod planning;
mod pulls;
mod records;
mod source;
use super::actions::command as actions;
use super::{Cause, Client, Error, Relation};
use crate::tool::Reply;
use anyhow::Result;
use clap::{Parser, Subcommand};
pub(super) use delivery::read;
use delivery::{Span, Spot};
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};
#[derive(Parser)]
#[command(
    name = "@github",
    about = "Native GitHub operations on an explicitly selected credential",
    after_help = "\
Conventions:
  Credentials: --token-file <PATH>, --token-stdin (one bounded line for profile-less CI),
               or profile GITHUB_TOKEN_FILE / GITHUB_TOKEN.
  Arguments:   positional <OWNER> <NAME>; --head/--base take bare branch names (no owner:).
               Commit comparison is available at top-level `compare` and `lifecycle ancestry`.
  Receipts:    `dispatch` returns workflow_run_id/run_url/html_url; `issues file` returns
               html_url; `issues open` returns rows[].url."
)]
struct Input {
    #[arg(
        long = "token-file",
        global = true,
        help = "Absolute private file; otherwise profile GITHUB_TOKEN_FILE or GITHUB_TOKEN"
    )]
    file: Option<PathBuf>,
    #[arg(
        long = "token-stdin",
        global = true,
        conflicts_with = "file",
        help = "Read one bounded token line from stdin for profile-less CI"
    )]
    stdin: bool,
    #[arg(long, global = true, help = "Per-request timeout in seconds (1..=300)")]
    timeout: Option<u64>,
    #[arg(long, global = true, help = "Write structured provider receipts")]
    json: bool,
    #[command(subcommand)]
    deed: Deed,
}
#[derive(Subcommand)]
enum Deed {
    #[command(about = "Read repository metadata or create an organization repository")]
    Repository(source::Repository),
    Project(source::Project),
    #[command(about = "List one page of visible Projects owned by an organization")]
    Projects(delivery::Organization),
    #[command(about = "Read one page of an organization Project’s items")]
    Items(delivery::Items),
    #[command(about = "Read one page of field definitions for an organization Project")]
    Fields(delivery::Items),
    #[command(about = "Read exact metadata for an organization Project")]
    Overview(delivery::Overview),
    #[command(about = "Observe and manage Project views, workflows and status updates")]
    Planning(planning::Project),
    #[command(about = "Read one page of repositories linked to a Project")]
    Repositories(delivery::Items),
    #[command(
        name = "project-structure",
        about = "Create or configure a Project, or link or unlink a repository",
        after_help = r#"Example file: {"operation":"configure","project":"PVT_example","settings":{"title":"Delivery"}}"#
    )]
    Structure(delivery::Structure),
    #[command(
        name = "field-schema",
        about = "Create, update, attach or remove one Project field",
        after_help = r#"Example file: {"operation":"update","field":"PVTF_example","name":"Priority"}"#
    )]
    Schema(delivery::Schema),
    #[command(about = "Read one page of values on an exact Project item")]
    Values(delivery::Values),
    #[command(
        name = "project-change",
        about = "Submit one typed Project item mutation from JSON",
        after_help = r#"Example file: {"project":"PVT_example","action":{"operation":"set","item":"PVTI_example","field":"PVTF_example","value":{"singleSelectOptionId":"option"}}}
Operations: add (content), remove (item), move (item, optional after), set (item, field, value), clear (item, field)."#
    )]
    Change(delivery::Change),
    #[command(name = "ref", about = "Read one exact Git branch or tag reference")]
    Pointer(source::Pointer),
    #[command(about = "Read one page of repository tags and their commits")]
    Tags(source::Tags),
    #[command(about = "Read one annotated Git tag object by complete SHA")]
    Annotation(source::Annotation),
    #[command(about = "Read one exact Git tree, optionally including descendants")]
    Tree(source::Tree),
    #[command(about = "Read a branch head and GitHub protection status")]
    Branch(source::Branch),
    #[command(about = "Read one user’s effective repository permission and role")]
    Permission(source::Permission),
    #[command(about = "Read a Git blob and verify its object identity")]
    Blob(source::Blob),
    #[command(about = "Read and verify an exact file in a complete commit SHA")]
    Contents(source::Contents),
    #[command(about = "Read exact Issue review comments and reopening history")]
    Acceptance(acceptance::Acceptance),
    #[command(about = "Read branches matching a literal Git ref prefix")]
    Matching(source::Matching),
    #[command(about = "Read GitHub release and asset metadata")]
    Releases(source::Releases),
    Close {
        node: String,
    },
    #[command(about = "Find one page of exact base and head matches within a repository")]
    Pulls(pulls::Selection),
    #[command(about = "Create one pull request between branches in the same repository")]
    Propose(pulls::Submission),
    #[command(about = "Update the title and body of one exact pull request")]
    Edit(pulls::Revision),
    #[command(about = "Read pull readiness, page by head or update a single pull field")]
    Lifecycle(pulls::Lifecycle),
    #[command(about = "Compare two complete commit SHAs for ancestry and status")]
    Compare(pulls::Ancestry),
    #[command(about = "Read the Git tree of a complete commit SHA")]
    Commit(delivery::Commit),
    #[command(about = "Read one pull request and its exact base and head")]
    Pull(Spot),
    #[command(about = "Squash one pull request conditional on its complete head SHA")]
    Squash(pulls::Landing),
    #[command(about = "Read one page of check runs for a complete commit SHA")]
    Checks(actions::Checks),
    #[command(about = "Read one workflow run or a specific attempt")]
    Workflow(actions::Workflow),
    #[command(about = "Dispatch one workflow and return its accepted run identity")]
    Dispatch(actions::Request),
    #[command(about = "Send a repository_dispatch event and verify 204 acceptance")]
    Trigger(actions::Trigger),
    #[command(about = "Read one page of jobs for an exact workflow run attempt")]
    Jobs(actions::Catalog),
    #[command(about = "Read bounded logs for an exact Actions job")]
    Logs(actions::Logs),
    #[command(about = "Read one exact issue and its repository identity")]
    Issue(Spot),
    Issues(records::Issues),
    #[command(
        name = "create-issue",
        about = "Create one issue in an exact repository node with an optional issue type"
    )]
    Create(delivery::Creation),
    #[command(about = "List one page of issue types defined by an organization")]
    Kinds(delivery::Organization),
    #[command(about = "Read one page of organization Issue field definitions")]
    Definitions(planning::Definitions),
    #[command(about = "Read one page of field values on an exact Issue")]
    Observations(planning::Observations),
    #[command(about = "Set or clear a single-select Issue field by its native IDs")]
    Phase(planning::Phase),
    #[command(about = "Read one page of combined commit statuses for a complete SHA")]
    Statuses(delivery::Statuses),
    #[command(about = "Write one named status for a complete commit SHA")]
    Mark(delivery::Mark),
    #[command(about = "List one page of Actions artifacts for a repository or workflow run")]
    Artifacts(delivery::Artifacts),
    #[command(about = "Download a verified Actions artifact ZIP into a new file")]
    Download(delivery::Download),
    #[command(about = "Read one exact Actions artifact's metadata")]
    Artifact(actions::Artifact),
    #[command(about = "Read one page of comments")]
    Comments(Span),
    #[command(about = "Read one page of labels")]
    Labels(Span),
    #[command(about = "Read one page of child issues or blockers")]
    Relations {
        #[arg(value_enum)]
        relation: Relation,
        #[command(flatten)]
        span: Span,
    },
    #[command(about = "Post one comment from a bounded UTF-8 file")]
    Comment {
        node: String,
        #[arg(long = "body-file")]
        body: PathBuf,
    },
    #[command(about = "Add one child or blocker relationship")]
    Link {
        #[arg(value_enum)]
        relation: Relation,
        node: String,
        related: String,
    },
}

pub(crate) fn call(argv: &[String], vars: &BTreeMap<String, String>) -> Result<Reply> {
    let input =
        Input::try_parse_from(std::iter::once("@github").chain(argv.iter().map(String::as_str)))?;
    Ok(Reply::plain("github", execute(input, vars)?))
}
fn apply(client: &Client, deed: Deed) -> Result<Value, Error> {
    let value = match deed {
        Deed::Repository(repository) => return repository.execute(client),
        Deed::Project(project) => return project.execute(client),
        Deed::Projects(projects) => return projects.projects(client),
        Deed::Tags(tags) => return tags.execute(client),
        Deed::Items(items) => return items.execute(client),
        Deed::Fields(fields) => return fields.fields(client),
        Deed::Overview(overview) => return overview.execute(client),
        Deed::Planning(planning) => return planning.execute(client),
        Deed::Repositories(repositories) => return repositories.repositories(client),
        Deed::Structure(structure) => return structure.execute(client),
        Deed::Schema(schema) => return schema.execute(client),
        Deed::Values(values) => return values.execute(client),
        Deed::Change(change) => return change.execute(client),
        Deed::Pointer(pointer) => return pointer.execute(client),
        Deed::Annotation(annotation) => return annotation.execute(client),
        Deed::Tree(tree) => return tree.execute(client),
        Deed::Branch(branch) => return branch.execute(client),
        Deed::Permission(permission) => return permission.execute(client),
        Deed::Blob(blob) => return blob.execute(client),
        Deed::Contents(contents) => return contents.execute(client),
        Deed::Acceptance(acceptance) => return acceptance.execute(client),
        Deed::Matching(matching) => return matching.execute(client),
        Deed::Releases(releases) => return releases.execute(client),
        Deed::Close { node } => serde_json::to_value(client.close(&node)?),
        Deed::Pulls(selection) => return selection.execute(client),
        Deed::Propose(submission) => return submission.execute(client),
        Deed::Edit(edit) => return edit.execute(client),
        Deed::Lifecycle(lifecycle) => return lifecycle.execute(client),
        Deed::Compare(compare) => return compare.execute(client),
        Deed::Commit(commit) => return commit.execute(client),
        Deed::Pull(place) => serde_json::to_value(client.pull(&place.target()?)?),
        Deed::Squash(landing) => return landing.execute(client),
        Deed::Checks(checks) => return checks.execute(client),
        Deed::Workflow(workflow) => return workflow.execute(client),
        Deed::Dispatch(request) => return request.execute(client),
        Deed::Trigger(trigger) => return trigger.execute(client),
        Deed::Jobs(catalog) => return catalog.execute(client),
        Deed::Logs(logs) => return logs.execute(client),
        Deed::Issues(issues) => return issues.execute(client),
        Deed::Issue(place) => serde_json::to_value(client.issue(&place.target()?)?),
        Deed::Create(creation) => return creation.execute(client),
        Deed::Kinds(kinds) => return kinds.execute(client),
        Deed::Definitions(definitions) => return definitions.execute(client),
        Deed::Observations(observations) => return observations.execute(client),
        Deed::Phase(phase) => return phase.execute(client),
        Deed::Statuses(statuses) => return statuses.execute(client),
        Deed::Mark(mark) => return mark.execute(client),
        Deed::Artifacts(artifacts) => return artifacts.execute(client),
        Deed::Download(download) => return download.execute(client),
        Deed::Artifact(artifact) => return artifact.execute(client),
        Deed::Comments(span) => {
            let (target, window) = span.parts()?;
            serde_json::to_value(client.comments(&target, &window)?)
        }
        Deed::Labels(span) => {
            let (target, window) = span.parts()?;
            serde_json::to_value(client.labels(&target, &window)?)
        }
        Deed::Relations { relation, span } => {
            let (target, window) = span.parts()?;
            serde_json::to_value(client.relations(&target, &window, relation)?)
        }
        Deed::Comment { node, body } => serde_json::to_value(client.comment(&node, &read(body)?)?),
        Deed::Link {
            relation,
            node,
            related,
        } => serde_json::to_value(client.link(&node, &related, relation)?),
    };
    value.map_err(|_| Error::local(Cause::Shape))
}
#[cfg(test)]
mod tests;
