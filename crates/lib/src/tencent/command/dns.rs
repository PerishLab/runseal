use crate::tencent::dns::{self, Operation, Request};
use anyhow::{Result, bail};
use clap::{Args, Subcommand, ValueEnum};
use serde_json::{Value, json};

#[derive(Subcommand)]
pub(in crate::tencent) enum Dns {
    Domain(Selection),
    #[command(subcommand)]
    Record(Record),
}

#[derive(Args)]
#[group(required = true, multiple = false)]
pub(in crate::tencent) struct Selection {
    #[arg(long)]
    domain: Option<String>,
    #[arg(long = "domain-id")]
    number: Option<u64>,
}

#[derive(Subcommand)]
pub(in crate::tencent) enum Record {
    Get {
        #[command(flatten)]
        zone: Selection,
        #[arg(long)]
        id: u64,
    },
    List {
        #[command(flatten)]
        zone: Selection,
        #[arg(long, default_value_t = 0)]
        offset: u32,
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
    Create(Fields),
    Modify {
        #[arg(long)]
        id: u64,
        #[command(flatten)]
        record: Fields,
    },
    Delete {
        #[command(flatten)]
        zone: Selection,
        #[arg(long)]
        id: u64,
    },
}

#[derive(Clone, Copy, ValueEnum)]
#[value(rename_all = "UPPER")]
pub(in crate::tencent) enum Kind {
    A,
    Aaaa,
    Cname,
    Txt,
}

#[derive(Args)]
pub(in crate::tencent) struct Fields {
    #[command(flatten)]
    zone: Selection,
    #[arg(long)]
    name: String,
    #[arg(long)]
    kind: Kind,
    #[arg(long)]
    value: String,
    #[arg(long, default_value = "默认")]
    line: String,
    #[arg(long, default_value_t = 600)]
    ttl: u32,
}

impl Selection {
    fn model(&self) -> Result<dns::Zone> {
        match (&self.domain, self.number) {
            (Some(domain), None) => Ok(dns::Zone::Name(domain.clone())),
            (None, Some(id)) => Ok(dns::Zone::Id(id)),
            _ => bail!("select exactly one domain name or DomainId"),
        }
    }
}

impl Fields {
    fn model(&self) -> dns::Fields {
        let kind = match self.kind {
            Kind::A => dns::Kind::A,
            Kind::Aaaa => dns::Kind::Aaaa,
            Kind::Cname => dns::Kind::Cname,
            Kind::Txt => dns::Kind::Txt,
        };
        dns::Fields {
            name: self.name.clone(),
            kind,
            value: self.value.clone(),
            line: self.line.clone(),
            ttl: self.ttl,
        }
    }
}

pub(in crate::tencent) fn request(deed: &Dns) -> Result<Request> {
    let (zone, operation) = match deed {
        Dns::Domain(zone) => (zone, Operation::Domain),
        Dns::Record(Record::Get { zone, id }) => (zone, Operation::Get { id: *id }),
        Dns::Record(Record::List {
            zone,
            offset,
            limit,
        }) => (
            zone,
            Operation::List {
                offset: *offset,
                limit: *limit,
            },
        ),
        Dns::Record(Record::Create(fields)) => (&fields.zone, Operation::Create(fields.model())),
        Dns::Record(Record::Modify { id, record }) => (
            &record.zone,
            Operation::Modify {
                id: *id,
                fields: record.model(),
            },
        ),
        Dns::Record(Record::Delete { zone, id }) => (zone, Operation::Delete { id: *id }),
    };
    Ok(Request {
        zone: zone.model()?,
        operation,
    })
}

pub(super) fn positive(value: u64) -> Result<Value> {
    if value == 0 {
        bail!("Tencent resource identifier must be positive");
    }
    Ok(json!(value))
}
