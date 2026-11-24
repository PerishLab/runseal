use super::{Call, Cause, Client, Error, Receipt, catalogue, graph, mutation, node};
use crate::github::{Identity, Outcome, field};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize, Serialize)]
pub struct Removed {
    pub id: String,
    pub project: Identity,
}

impl Client {
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
