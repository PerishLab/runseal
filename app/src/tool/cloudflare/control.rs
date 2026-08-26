use anyhow::Result;

use super::{Seat, api::Client, deed::Control, token::segment};
use crate::tool::Reply;

pub(super) fn act(seat: &Seat, client: &Client, deed: &Control) -> Result<Reply> {
    let account = seat.account()?;
    let (method, route, kind, body) = match deed {
        Control::Worker(name) => (
            "GET",
            format!("{account}/workers/services/{}", segment(name)),
            "worker",
            None,
        ),
        Control::Domains => ("GET", format!("{account}/workers/domains"), "domains", None),
        Control::Bucket(name) => (
            "GET",
            format!("{account}/r2/buckets/{}", segment(name)),
            "bucket",
            None,
        ),
        Control::Create(name) => (
            "POST",
            format!("{account}/r2/buckets"),
            "bucket",
            Some(serde_json::json!({ "name": name })),
        ),
        Control::Custom(name) => (
            "GET",
            format!("{account}/r2/buckets/{}/domains/custom", segment(name)),
            "domains",
            None,
        ),
        Control::Attach(name) => (
            "POST",
            format!("{account}/r2/buckets/{}/domains/custom", segment(name)),
            "domain",
            Some(seat.body()?),
        ),
        Control::Normalize { bucket, domain } => (
            "PUT",
            format!(
                "{account}/r2/buckets/{}/domains/custom/{}",
                segment(bucket),
                segment(domain)
            ),
            "domain",
            Some(seat.body()?),
        ),
        Control::Detach { bucket, domain } => (
            "DELETE",
            format!(
                "{account}/r2/buckets/{}/domains/custom/{}",
                segment(bucket),
                segment(domain)
            ),
            "domain",
            None,
        ),
        Control::Drop(name) => (
            "DELETE",
            format!("{account}/r2/buckets/{}", segment(name)),
            "bucket",
            None,
        ),
    };
    let page = client.send(method, &route, body.as_ref())?;
    Ok(Reply::plain(kind, page.result))
}
