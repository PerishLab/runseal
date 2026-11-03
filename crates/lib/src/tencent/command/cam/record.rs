use super::{Data, Entity, Page, Receipt, Relation, Request, model};
use crate::tencent::{Class, Failure, Identity, Outcome, api};
use serde_json::Value;
use std::collections::HashSet;

fn response() -> Failure {
    Failure::new(
        Class::Response,
        "CAM policy receipt malformed, conflicting or outside its finite model",
    )
}

pub(super) fn receipt(
    request: &Request,
    identity: Identity,
    value: &Value,
) -> Result<Receipt, Failure> {
    let checked = api::project::<model::Ack>(value)
        .map_err(|error| Failure::capture(error, Class::Response))?;
    let id = checked["RequestId"].as_str().expect("checked request ID");
    let data = Reading(value).data(request).map_err(|mut fault| {
        fault.request = Some(id.into());
        fault
    })?;
    Ok(Receipt {
        identity,
        service: "cam".into(),
        version: "2019-01-16".into(),
        action: request.action().into(),
        region: String::new(),
        observed: crate::tencent::model::now(),
        request: id.into(),
        outcome: match request {
            Request::Entities { .. } => Outcome::Observed,
            Request::Delete { .. } => Outcome::Accepted,
        },
        data,
    })
}

struct Reading<'a>(&'a Value);

impl Reading<'_> {
    fn data(&self, request: &Request) -> Result<Data, Failure> {
        let value = self.0;
        if let Some(id) = value.get("PolicyId")
            && id.as_u64() != Some(request.policy())
        {
            return Err(response());
        }
        match request {
            Request::Entities { policy, page } => {
                self.associations(*policy, *page).map(Data::Entities)
            }
            Request::Delete { policy } => Ok(Data::Deleted {
                policy: *policy,
                complete: self.covered(&["RequestId"]),
            }),
        }
    }

    fn covered(&self, fields: &[&str]) -> bool {
        self.0
            .as_object()
            .is_some_and(|object| object.keys().all(|key| fields.contains(&key.as_str())))
    }

    fn associations(&self, policy: u64, page: u32) -> Result<Page, Failure> {
        let value = self.0;
        let total = value["TotalNum"].as_u64().ok_or_else(response)?;
        let rows = value["List"].as_array().ok_or_else(response)?;
        let offset = u64::from(page - 1) * 100;
        if rows.len() as u64 != total.saturating_sub(offset).min(100) {
            return Err(response());
        }
        let mut complete = self.covered(&["List", "TotalNum", "RequestId"]);
        let mut seen = HashSet::new();
        let mut items = Vec::new();
        for row in rows {
            let (entity, checked) = Reading(row).entity()?;
            complete &= checked;
            if !seen.insert((entity.relation, entity.id.clone())) {
                return Err(response());
            }
            items.push(entity);
        }
        let next = if offset + (items.len() as u64) < total {
            Some(page.checked_add(1).ok_or_else(response)?)
        } else {
            None
        };
        Ok(Page {
            policy,
            page,
            limit: 100,
            total,
            next,
            items,
            complete,
        })
    }

    fn entity(&self) -> Result<(Entity, bool), Failure> {
        let value = self.0;
        let id = value["Id"].as_str().ok_or_else(response)?;
        if id.is_empty() || id.len() > 64 || !id.bytes().all(|b| b.is_ascii_digit()) {
            return Err(response());
        }
        let relation = match self.optional("RelatedType")? {
            Some(1) => Some(Relation::User),
            Some(2) => Some(Relation::Group),
            Some(3) => Some(Relation::Role),
            _ => None,
        };
        let uin = self.optional("Uin")?;
        let known = self.covered(&["Id", "RelatedType", "Uin", "Name", "AttachmentTime"]);
        let mut complete = known && relation.is_some() && uin.is_some();
        for field in ["Name", "AttachmentTime"] {
            if let Some(value) = value.get(field) {
                complete &= value.is_null() || value.as_str().is_some_and(|s| s.len() <= 1024);
            }
        }
        Ok((
            Entity {
                id: id.into(),
                relation,
                uin,
            },
            complete,
        ))
    }

    fn optional(&self, field: &str) -> Result<Option<u64>, Failure> {
        match self.0.get(field) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => value.as_u64().map(Some).ok_or_else(response),
        }
    }
}
