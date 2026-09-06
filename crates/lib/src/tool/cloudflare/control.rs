use anyhow::Result;

use super::{
    Seat,
    api::Client,
    deed::{Bucket, Control, Domain, Worker},
    token::segment,
};
use crate::tool::Reply;

pub(super) fn act(seat: &Seat, client: &Client, deed: &Control) -> Result<Reply> {
    let account = seat.account()?;
    let (method, route, kind, body) = match deed {
        Control::Worker(Worker::Service(name)) => (
            "GET",
            format!("{account}/workers/services/{}", segment(name)),
            "worker",
            None,
        ),
        Control::Worker(Worker::Domains) => {
            ("GET", format!("{account}/workers/domains"), "domains", None)
        }
        Control::Bucket(Bucket::Show(name)) => (
            "GET",
            format!("{account}/r2/buckets/{}", segment(name)),
            "bucket",
            None,
        ),
        Control::Bucket(Bucket::Create(name)) => (
            "POST",
            format!("{account}/r2/buckets"),
            "bucket",
            Some(serde_json::json!({ "name": name })),
        ),
        Control::Bucket(Bucket::Domain(Domain::List(name))) => (
            "GET",
            format!("{account}/r2/buckets/{}/domains/custom", segment(name)),
            "domains",
            None,
        ),
        Control::Bucket(Bucket::Domain(Domain::Create(name))) => (
            "POST",
            format!("{account}/r2/buckets/{}/domains/custom", segment(name)),
            "domain",
            Some(seat.body()?),
        ),
        Control::Bucket(Bucket::Domain(Domain::Edit { bucket, domain })) => (
            "PUT",
            format!(
                "{account}/r2/buckets/{}/domains/custom/{}",
                segment(bucket),
                segment(domain)
            ),
            "domain",
            Some(seat.body()?),
        ),
        Control::Bucket(Bucket::Domain(Domain::Drop { bucket, domain })) => (
            "DELETE",
            format!(
                "{account}/r2/buckets/{}/domains/custom/{}",
                segment(bucket),
                segment(domain)
            ),
            "domain",
            None,
        ),
        Control::Bucket(Bucket::Drop(name)) => (
            "DELETE",
            format!("{account}/r2/buckets/{}", segment(name)),
            "bucket",
            None,
        ),
    };
    let page = client.send(method, &route, body.as_ref())?;
    Ok(Reply::plain(kind, page.result))
}
