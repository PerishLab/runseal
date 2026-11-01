use super::super::Class;
use super::{Data, Product, Query, Window};
use serde_json::{Value, json};

fn read(product: Product, fields: Value) -> Result<Data, super::super::Failure> {
    let id = if product == Product::Cvm {
        "ins-one"
    } else {
        "lhins-one"
    };
    let mut instance = json!({"InstanceId":id,"InstanceState":"RUNNING","Zone":"na-ashburn-1"});
    instance
        .as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    Query::Instances {
        product,
        region: "na-ashburn".into(),
        window: Window {
            offset: 0,
            limit: 1,
        },
    }
    .call()
    .unwrap()
    .decode(&json!({"TotalCount":1,"InstanceSet":[instance]}))
}

#[test]
fn native() {
    for (product, public, private) in [
        (Product::Cvm, "PublicIpAddresses", "PrivateIpAddresses"),
        (Product::Lighthouse, "PublicAddresses", "PrivateAddresses"),
    ] {
        let Data::Instances(page) = read(
            product,
            json!({(public):["192.0.2.1","2001:db8::1"],(private):["10.0.0.1"]}),
        )
        .unwrap() else {
            panic!("instances")
        };
        assert_eq!(
            page.items[0]
                .public
                .as_ref()
                .unwrap()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            ["192.0.2.1", "2001:db8::1"]
        );
        assert_eq!(
            page.items[0].private.as_ref().unwrap()[0].to_string(),
            "10.0.0.1"
        );
        for fields in [json!({}), json!({(public):null,(private):null})] {
            let Data::Instances(page) = read(product, fields).unwrap() else {
                panic!("instances")
            };
            assert_eq!(page.items[0].public, None);
            assert_eq!(page.items[0].private, None);
        }
        let Data::Instances(page) = read(product, json!({(public):[],(private):[]})).unwrap()
        else {
            panic!("instances")
        };
        assert_eq!(page.items[0].public, Some(vec![]));
        assert_eq!(page.items[0].private, Some(vec![]));
        for fields in [
            json!({(public):["invalid"]}),
            json!({(private):["invalid"]}),
            json!({(public):"192.0.2.1"}),
            json!({(private):[1]}),
        ] {
            assert_eq!(read(product, fields).unwrap_err().class, Class::Response);
        }
    }
}

#[test]
fn foreign() {
    for (product, public, private) in [
        (Product::Cvm, "PublicAddresses", "PrivateAddresses"),
        (
            Product::Lighthouse,
            "PublicIpAddresses",
            "PrivateIpAddresses",
        ),
    ] {
        for fields in [
            json!({(public):[]}),
            json!({(private):null}),
            json!({"PublicAddresses":[],"PublicIpAddresses":[]}),
        ] {
            assert_eq!(read(product, fields).unwrap_err().class, Class::Response);
        }
    }
}
