use super::super::{Class, Failure};
use super::query::{Call, Kind, Product, Window, token};
use super::types::{Bundle, Data, Image, Instance, Page, Region, Shape};
use serde::de::DeserializeOwned;
use serde_json::Value;

impl Call {
    pub(in crate::tencent) fn decode(&self, value: &Value) -> Result<Data, Failure> {
        let receipt = Receipt(value);
        match self.kind {
            Kind::Regions => {
                let items: Vec<Region> = receipt.counted("RegionSet")?;
                for item in &items {
                    token(&item.id).map_err(|_| response())?;
                }
                Ok(Data::Regions(items))
            }
            Kind::Zones => {
                let field = if self.product == Product::Lighthouse {
                    "ZoneInfoSet"
                } else {
                    "ZoneSet"
                };
                let items: Vec<Region> = receipt.counted(field)?;
                for item in &items {
                    self.zone(&item.id)?;
                }
                Ok(Data::Zones(items))
            }
            Kind::Instances(product, window) => {
                let items: Vec<Instance> = receipt.list("InstanceSet")?;
                let items = items
                    .into_iter()
                    .map(|item| self.instance(item, product))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Data::Instances(receipt.page(window, items)?))
            }
            Kind::Shapes => {
                let items: Vec<Shape> = receipt.list("InstanceTypeQuotaSet")?;
                for item in &items {
                    token(&item.id).map_err(|_| response())?;
                    self.zone(&item.zone)?;
                }
                Ok(Data::Shapes(items))
            }
            Kind::Images(window) => {
                let items = receipt.images("ImageSet", "img-")?;
                Ok(Data::Images(receipt.page(window, items)?))
            }
            Kind::Blueprints(window) => {
                let items = receipt.images("BlueprintSet", "lhbp-")?;
                Ok(Data::Blueprints(receipt.page(window, items)?))
            }
            Kind::Bundles(window) => {
                let items: Vec<Bundle> = receipt.list("BundleSet")?;
                for item in &items {
                    token(&item.id).map_err(|_| response())?;
                }
                Ok(Data::Bundles(receipt.page(window, items)?))
            }
        }
    }

    fn instance(&self, wire: Instance, product: Product) -> Result<Instance, Failure> {
        identifier(
            &wire.id,
            if product == Product::Cvm {
                "ins-"
            } else {
                "lhins-"
            },
        )?;
        if wire.state.is_empty() {
            return Err(response());
        }
        let instance = wire.normalize()?;
        if let Some(zone) = &instance.zone {
            self.zone(zone)?;
        }
        Ok(instance)
    }

    fn zone(&self, value: &str) -> Result<(), Failure> {
        token(value).map_err(|_| response())?;
        if self
            .region
            .as_ref()
            .is_none_or(|region| !value.starts_with(&format!("{region}-")))
        {
            return Err(response());
        }
        Ok(())
    }
}

struct Receipt<'a>(&'a Value);

impl Receipt<'_> {
    fn images(&self, field: &str, prefix: &str) -> Result<Vec<Image>, Failure> {
        let items: Vec<Image> = if field == "BlueprintSet" {
            self.0[field]
                .as_array()
                .ok_or_else(response)?
                .iter()
                .map(Image::blueprint)
                .collect::<Result<_, _>>()?
        } else {
            self.list(field)?
        };
        for item in &items {
            identifier(&item.id, prefix)?;
        }
        Ok(items)
    }

    fn list<T: DeserializeOwned>(&self, field: &str) -> Result<Vec<T>, Failure> {
        let list = self
            .0
            .get(field)
            .filter(|value| value.is_array())
            .ok_or_else(response)?;
        serde_json::from_value(list.clone()).map_err(|_| response())
    }

    fn counted<T: DeserializeOwned>(&self, field: &str) -> Result<Vec<T>, Failure> {
        let items = self.list(field)?;
        if self.0["TotalCount"].as_u64() != Some(items.len() as u64) {
            return Err(response());
        }
        Ok(items)
    }

    fn page<T>(&self, window: Window, items: Vec<T>) -> Result<Page<T>, Failure> {
        let total = self
            .0
            .get("TotalCount")
            .and_then(Value::as_u64)
            .ok_or_else(response)?;
        let count = u32::try_from(items.len()).map_err(|_| response())?;
        if count > window.limit || (count == 0 && total > u64::from(window.offset)) {
            return Err(response());
        }
        let end = window.offset.checked_add(count).ok_or_else(response)?;
        if count > 0 && u64::from(end) > total {
            return Err(response());
        }
        let next = (count > 0 && u64::from(end) < total).then_some(end);
        Ok(Page {
            items,
            total,
            offset: window.offset,
            limit: window.limit,
            next,
        })
    }
}

fn identifier(value: &str, prefix: &str) -> Result<(), Failure> {
    token(value).map_err(|_| response())?;
    if !value.starts_with(prefix) || value.len() == prefix.len() {
        return Err(response());
    }
    Ok(())
}

fn response() -> Failure {
    Failure::new(
        Class::Response,
        "Tencent compute receipt is incomplete or outside the requested product, region or page",
    )
}
