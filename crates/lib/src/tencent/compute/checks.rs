use super::super::Class;
use super::{Data, Product, Query, Window};
use serde_json::json;

fn query(product: Product) -> Query {
    Query::Instances {
        product,
        region: "ap-hongkong".into(),
        window: Window {
            offset: 0,
            limit: 1,
        },
    }
}

#[test]
fn pages() {
    let call = query(Product::Cvm).call().unwrap();
    let value = json!({"TotalCount":2,"InstanceSet":[{"InstanceId":"ins-one","InstanceState":"RUNNING","Placement":{"Zone":"ap-hongkong-1"},"PublicIpAddresses":[],"SecretKey":"SECRET_FIXTURE","Password":"SECRET_FIXTURE"}]});
    let data = call.decode(&value).unwrap();
    let text = serde_json::to_string(&data).unwrap();
    assert!(!text.contains("SECRET_FIXTURE"));
    assert!(!text.contains("Placement"));
    let Data::Instances(page) = data else {
        panic!("instances")
    };
    assert_eq!(page.next, Some(1));
    assert_eq!(page.total, 2);
    assert_eq!(page.items[0].zone.as_deref(), Some("ap-hongkong-1"));
    assert_eq!(page.items[0].public, Some(vec![]));
    assert_eq!(page.items[0].private, None);
    let Data::Instances(empty) = call
        .decode(&json!({"TotalCount":0,"InstanceSet":[]}))
        .unwrap()
    else {
        panic!("empty")
    };
    assert!(empty.items.is_empty());
    assert_eq!(empty.next, None);
}

#[test]
fn scope() {
    let call = query(Product::Lighthouse).call().unwrap();
    let value = json!({"TotalCount":1,"InstanceSet":[{"InstanceId":"lhins-one","InstanceState":"RUNNING","Zone":"ap-hongkong-2","PublicIpAddresses":["192.0.2.1"]}]});
    assert!(call.decode(&value).is_ok());
    for value in [
        json!({"TotalCount":1,"InstanceSet":[{"InstanceId":"ins-one","InstanceState":"RUNNING"}]}),
        json!({"TotalCount":1,"InstanceSet":[{"InstanceId":"lhins-one","InstanceState":"RUNNING","Zone":"ap-shanghai-1"}]}),
        json!({"TotalCount":1,"InstanceSet":[{"InstanceId":"lhins-one","InstanceState":"RUNNING","Zone":"ap-hongkong-1","Placement":{"Zone":"ap-hongkong-2"}}]}),
        json!({"TotalCount":1,"InstanceSet":[{"InstanceId":"lhins-one","InstanceState":"RUNNING","PublicIpAddresses":["invalid"]}]}),
    ] {
        assert_eq!(call.decode(&value).unwrap_err().class, Class::Response);
    }
}

#[test]
fn incomplete() {
    let call = query(Product::Cvm).call().unwrap();
    for value in [
        json!({"InstanceSet":[]}),
        json!({"TotalCount":0,"InstanceSet":null}),
        json!({"TotalCount":10,"InstanceSet":[]}),
        json!({"TotalCount":0,"InstanceSet":[{"InstanceId":"ins-one","InstanceState":"RUNNING"}]}),
        json!({"TotalCount":2,"InstanceSet":[{"InstanceId":"ins-one","InstanceState":"RUNNING"},{"InstanceId":"ins-two","InstanceState":"RUNNING"}]}),
        json!({"TotalCount":1,"InstanceSet":[{"InstanceId":"ins-one"}]}),
    ] {
        assert_eq!(call.decode(&value).unwrap_err().class, Class::Response);
    }
}

#[test]
fn catalogs() {
    let window = Window {
        offset: 0,
        limit: 20,
    };
    let queries = [
        (
            Query::Regions {
                product: Product::Cvm,
            },
            json!({"TotalCount":1,"RegionSet":[{"Region":"ap-hongkong","RegionName":"HK","RegionState":"AVAILABLE"}]}),
        ),
        (
            Query::Zones {
                product: Product::Lighthouse,
                region: "ap-hongkong".into(),
            },
            json!({"TotalCount":1,"ZoneInfoSet":[{"Zone":"ap-hongkong-1","ZoneName":"HK1","InstanceDisplayLabel":"NORMAL"}]}),
        ),
        (
            Query::Shapes {
                region: "ap-hongkong".into(),
                zone: None,
            },
            json!({"InstanceTypeQuotaSet":[{"InstanceType":"S5.SMALL2","Zone":"ap-hongkong-1","Cpu":1,"Memory":2,"Status":"SELL"}]}),
        ),
        (
            Query::Images {
                region: "ap-hongkong".into(),
                window,
            },
            json!({"TotalCount":1,"ImageSet":[{"ImageId":"img-one","ImageName":"Linux","ImageState":"NORMAL"}]}),
        ),
        (
            Query::Blueprints {
                region: "ap-hongkong".into(),
                window,
            },
            json!({"TotalCount":1,"BlueprintSet":[{"BlueprintId":"lhbp-one","ImageId":"img-underlying","BlueprintName":"Linux","BlueprintState":"NORMAL","Platform":"UBUNTU","PlatformType":"LINUX_UNIX"}]}),
        ),
        (
            Query::Bundles {
                region: "ap-hongkong".into(),
                blueprint: None,
                window,
            },
            json!({"TotalCount":1,"BundleSet":[{"BundleId":"bundle_one","CPU":2,"Memory":4}]}),
        ),
    ];
    for (query, value) in queries {
        assert!(query.call().unwrap().decode(&value).is_ok(), "{query:?}");
    }
}
