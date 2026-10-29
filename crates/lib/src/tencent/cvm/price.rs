use super::{Failure, Spec, names::Names, record::Record, response};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Serialize)]
pub struct Tier {
    pub price: Option<f64>,
    pub discounted: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Rate {
    pub unit: String,
    pub price: f64,
    pub discounted: Option<f64>,
    pub second: Option<Tier>,
    pub third: Option<Tier>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Price {
    pub spec: Spec,
    pub currency: Option<String>,
    pub instance: Rate,
    pub bandwidth: Option<Rate>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Quota {
    pub zone: String,
    pub used: u64,
    pub remaining: u64,
    pub total: u64,
}

impl Record<'_> {
    fn amount(&self, key: &str) -> Result<Option<f64>, Failure> {
        self.0
            .get(key)
            .filter(|v| !v.is_null())
            .map(|value| {
                value
                    .as_f64()
                    .filter(|v| v.is_finite() && *v >= 0.0)
                    .ok_or_else(response)
            })
            .transpose()
    }

    fn tier(&self, names: (&str, &str)) -> Result<Option<Tier>, Failure> {
        let price = self.amount(names.0)?;
        let discounted = self.amount(names.1)?;
        Ok(if price.is_none() && discounted.is_none() {
            None
        } else {
            Some(Tier { price, discounted })
        })
    }

    fn rate(&self, unit: &str) -> Result<Rate, Failure> {
        let observed = self.text("ChargeUnit", 16)?;
        if observed != unit {
            return Err(response());
        }
        Ok(Rate {
            unit: observed,
            price: self.amount("UnitPrice")?.ok_or_else(response)?,
            discounted: self.amount("UnitPriceDiscount")?,
            second: self.tier(("UnitPriceSecondStep", "UnitPriceDiscountSecondStep"))?,
            third: self.tier(("UnitPriceThirdStep", "UnitPriceDiscountThirdStep"))?,
        })
    }

    pub(super) fn price(&self, spec: &Spec) -> Result<Price, Failure> {
        let price = &self.0["Price"];
        let instance = Record(&price["InstancePrice"]).rate("HOUR")?;
        let bandwidth = price
            .get("BandwidthPrice")
            .filter(|v| !v.is_null())
            .map(|v| Record(v).rate("GB"))
            .transpose()?;
        if spec.connection.public && bandwidth.is_none() {
            return Err(response());
        }
        Ok(Price {
            spec: spec.clone(),
            currency: None,
            instance,
            bandwidth,
        })
    }
}

pub(super) fn quotas(
    value: &Value,
    region: &str,
    zone: Option<&str>,
) -> Result<Vec<Quota>, Failure> {
    let record = Record(&value["AccountQuotaOverview"]);
    if record.text("Region", 64)? != region {
        return Err(response());
    }
    let values = record.0["AccountQuota"]["PostPaidQuotaSet"]
        .as_array()
        .filter(|v| v.len() <= 100)
        .ok_or_else(response)?;
    let mut seen = std::collections::BTreeSet::new();
    let mut quotas = Vec::new();
    for value in values {
        let record = Record(value);
        let observed = record.text("Zone", 64)?;
        Names::zone(&observed, region).map_err(|_| response())?;
        if zone.is_some_and(|zone| zone != observed) || !seen.insert(observed.clone()) {
            return Err(response());
        }
        let quota = Quota {
            zone: observed,
            used: record.number("UsedQuota")?,
            remaining: record.number("RemainingQuota")?,
            total: record.number("TotalQuota")?,
        };
        if quota.used > quota.total || quota.remaining > quota.total {
            return Err(response());
        }
        quotas.push(quota);
    }
    Ok(quotas)
}
