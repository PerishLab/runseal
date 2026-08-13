use anyhow::Result;

use super::{Seat, api::Client, deed::Control, token::segment};
use crate::tool::Reply;

pub(super) fn act(seat: &Seat, client: &Client, deed: &Control) -> Result<Reply> {
    let account = seat.account()?;
    let (method, route, kind) = match deed {
        Control::Worker(name) => (
            "GET",
            format!("{account}/workers/services/{}", segment(name)),
            "worker",
        ),
        Control::Domains => ("GET", format!("{account}/workers/domains"), "domains"),
        Control::Bucket(name) => (
            "GET",
            format!("{account}/r2/buckets/{}", segment(name)),
            "bucket",
        ),
        Control::Custom(name) => (
            "GET",
            format!("{account}/r2/buckets/{}/domains/custom", segment(name)),
            "domains",
        ),
        Control::Detach { bucket, domain } => (
            "DELETE",
            format!(
                "{account}/r2/buckets/{}/domains/custom/{}",
                segment(bucket),
                segment(domain)
            ),
            "domain",
        ),
        Control::Drop(name) => (
            "DELETE",
            format!("{account}/r2/buckets/{}", segment(name)),
            "bucket",
        ),
    };
    let page = client.send(method, &route, None)?;
    Ok(Reply::plain(kind, page.result))
}
