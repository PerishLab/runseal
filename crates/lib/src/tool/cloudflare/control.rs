use anyhow::Result;

use super::{
    Seat,
    api::Client,
    deed::{Bucket, Control, Domain, Worker},
    token::segment,
};
use crate::tool::Reply;

struct Request {
    method: &'static str,
    route: String,
    kind: &'static str,
    body: Option<serde_json::Value>,
}

pub(super) fn act(seat: &Seat, client: &Client, deed: &Control) -> Result<Reply> {
    let account = seat.account()?;
    let request = match deed {
        Control::Worker(Worker::Service(name)) => Request {
            method: "GET",
            route: format!("{account}/workers/services/{}", segment(name)),
            kind: "worker",
            body: None,
        },
        Control::Worker(Worker::Domains) => Request {
            method: "GET",
            route: format!("{account}/workers/domains"),
            kind: "domains",
            body: None,
        },
        Control::Bucket(Bucket::Show(name)) => Request {
            method: "GET",
            route: format!("{account}/r2/buckets/{}", segment(name)),
            kind: "bucket",
            body: None,
        },
        Control::Bucket(Bucket::Create(name)) => Request {
            method: "POST",
            route: format!("{account}/r2/buckets"),
            kind: "bucket",
            body: Some(serde_json::json!({ "name": name })),
        },
        Control::Bucket(Bucket::Domain(Domain::List(name))) => Request {
            method: "GET",
            route: format!("{account}/r2/buckets/{}/domains/custom", segment(name)),
            kind: "domains",
            body: None,
        },
        Control::Bucket(Bucket::Domain(Domain::Create(name))) => Request {
            method: "POST",
            route: format!("{account}/r2/buckets/{}/domains/custom", segment(name)),
            kind: "domain",
            body: Some(seat.body()?),
        },
        Control::Bucket(Bucket::Domain(Domain::Edit { bucket, domain })) => Request {
            method: "PUT",
            route: format!(
                "{account}/r2/buckets/{}/domains/custom/{}",
                segment(bucket),
                segment(domain)
            ),
            kind: "domain",
            body: Some(seat.body()?),
        },
        Control::Bucket(Bucket::Domain(Domain::Drop { bucket, domain })) => Request {
            method: "DELETE",
            route: format!(
                "{account}/r2/buckets/{}/domains/custom/{}",
                segment(bucket),
                segment(domain)
            ),
            kind: "domain",
            body: None,
        },
        Control::Bucket(Bucket::Drop(name)) => Request {
            method: "DELETE",
            route: format!("{account}/r2/buckets/{}", segment(name)),
            kind: "bucket",
            body: None,
        },
    };
    let page = client.send(request.method, &request.route, request.body.as_ref())?;
    Ok(Reply::plain(request.kind, page.result))
}
