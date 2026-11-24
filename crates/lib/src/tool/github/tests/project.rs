use super::{reply, serve};
use crate::github::{Cause, Outcome, Window};
use serde_json::{Value, json};

const ITEM: &str = r#"{"id":"PVTI_fixture","kind":"ISSUE","archived":false,"updated":"2026-11-23T00:00:00Z","project":{"id":"PVT_example"},"content":{"id":"I_anchor","kind":"Issue","number":229,"title":"Delivery","url":"https://github.com/PerishLab/runseal/issues/229","repository":{"id":"R_example","name":"PerishLab/runseal"}}}"#;

fn page(row: &str) -> String {
    format!(
        r#"{{"data":{{"organization":{{"login":"PerishLab","projectV2":{{"id":"PVT_example","number":6,"list":{{"total":2,"rows":[{row}],"page":{{"more":true,"cursor":"next-page"}}}}}}}}}}}}"#
    )
}

#[test]
fn membership() {
    let good = page(ITEM);
    let wrong = page(&ITEM.replace("PVT_example", "PVT_other"));
    let server = serve(vec![
        reply("200 OK", "", &good),
        reply("200 OK", "", &wrong),
    ]);
    let window = Window::new(1, None).unwrap();
    let result = server.client.items("perishlab", 6, &window).unwrap();
    assert_eq!(result.value.target, "PVT_example");
    assert_eq!(result.value.next.as_deref(), Some("next-page"));
    assert_eq!(
        result.value.rows[0].content.as_ref().unwrap().number,
        Some(229)
    );
    let error = server.client.items("PerishLab", 6, &window).unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    let requests = server.thread.join().unwrap();
    assert!(requests[0].contains("projectV2(number:$number)"));
    assert!(requests[0].contains("\"number\":6"));
}

fn mutation(action: crate::github::Action) -> crate::github::Change {
    crate::github::Change {
        project: "PVT_example".into(),
        action,
    }
}

#[test]
fn write() {
    use crate::github::{Action, Field, Outcome};

    let item =
        r#"{"data":{"change":{"item":{"id":"PVTI_example","project":{"id":"PVT_example"}}}}}"#;
    let foreign = item.replace("PVT_example", "PVT_foreign");
    let server = serve(vec![
        reply("200 OK", "", item),
        reply("200 OK", "", &foreign),
        None,
    ]);
    let change = mutation(Action::Set {
        item: "PVTI_example".into(),
        field: "PVTF_example".into(),
        value: Field::Select("option".into()),
    });
    let receipt = server.client.change(&change).unwrap();
    assert_eq!(receipt.outcome, Outcome::Accepted);
    assert_eq!(receipt.value.item, "PVTI_example");
    for cause in [Cause::Shape, Cause::Network] {
        let error = server.client.change(&change).unwrap_err();
        assert_eq!(error.outcome, Outcome::Unknown);
        assert_eq!(error.cause, cause);
        assert_eq!(error.evidence.attempts, 1);
    }
    let requests = server.thread.join().unwrap();
    assert_eq!(requests.len(), 3);
    for input in [
        "updateProjectV2ItemFieldValue",
        "\"projectId\":\"PVT_example\"",
        "\"singleSelectOptionId\":\"option\"",
    ] {
        assert!(requests[0].contains(input));
    }
}

#[test]
fn operations() {
    use crate::github::{Action, Outcome};

    let add = r#"{"data":{"change":{"item":{"id":"PVTI_example","project":{"id":"PVT_example"},"content":{"id":"I_example"}}}}}"#;
    let remove = r#"{"data":{"change":{"deletedItemId":"PVTI_example"}}}"#;
    let order = r#"{"data":{"change":{"items":{"totalCount":7}}}}"#;
    let clear =
        r#"{"data":{"change":{"item":{"id":"PVTI_example","project":{"id":"PVT_example"}}}}}"#;
    let server = serve(vec![
        reply("200 OK", "", add),
        reply("200 OK", "", remove),
        reply("200 OK", "", order),
        reply("200 OK", "", clear),
    ]);
    let item = || "PVTI_example".to_owned();
    for action in [
        Action::Add {
            content: "I_example".into(),
        },
        Action::Remove { item: item() },
        Action::Move {
            item: item(),
            after: None,
        },
        Action::Clear {
            item: item(),
            field: "PVTF_example".into(),
        },
    ] {
        let receipt = server.client.change(&mutation(action)).unwrap();
        assert_eq!(receipt.outcome, Outcome::Accepted);
        assert_eq!(receipt.value.item, item());
    }
    let requests = server.thread.join().unwrap();
    assert!(requests[0].contains("addProjectV2ItemById"));
    assert!(requests[1].contains("deleteProjectV2Item"));
    assert!(requests[2].contains("\"afterId\":null"));
    assert!(requests[3].contains("clearProjectV2ItemFieldValue"));
}

#[test]
fn values() {
    let page = r#"{"data":{"node":{"id":"PVTI_example","project":{"id":"PVT_example"},"list":{"total":2,"rows":[{"kind":"ProjectV2ItemFieldNumberValue","field":{"id":"PVTF_number"},"number":0},{"kind":"FutureFieldValue"}],"page":{"more":false,"cursor":null}}}}}"#;
    let foreign = page.replace("PVT_example", "PVT_foreign");
    let server = serve(vec![
        reply("200 OK", "", page),
        reply("200 OK", "", &foreign),
    ]);
    let window = Window::new(2, None).unwrap();
    let receipt = server
        .client
        .values("PVT_example", "PVTI_example", &window)
        .unwrap();
    assert_eq!(receipt.value.rows[0].number, Some(0.0));
    assert_eq!(receipt.value.rows[1].kind, "FutureFieldValue");
    assert!(receipt.value.rows[1].field.is_none());
    let error = server
        .client
        .values("PVT_example", "PVTI_example", &window)
        .unwrap_err();
    assert_eq!(error.cause, Cause::Shape);
    assert!(server.thread.join().unwrap()[0].contains("fieldValues(first:$size,after:$after)"));
}

#[test]
fn fields() {
    let field = r#"{"id":"PVTF_fixture","name":"Priority","kind":"ProjectV2SingleSelectField","type":"SINGLE_SELECT","issue":false,"project":{"id":"PVT_example"},"options":[{"id":"option","name":"High","color":"RED","description":""}]}"#;
    let good = page(field);
    let foreign = page(&field.replace("PVT_example", "PVT_foreign"));
    let server = serve(vec![
        reply("200 OK", "", &good),
        reply("200 OK", "", &foreign),
    ]);
    let window = Window::new(1, None).unwrap();
    let receipt = server.client.fields("PerishLab", 6, &window).unwrap();
    let definition = &receipt.value.rows[0];
    assert_eq!(definition.name, "Priority");
    assert_eq!(definition.options.as_ref().unwrap()[0].id, "option");
    assert_eq!(
        server
            .client
            .fields("PerishLab", 6, &window)
            .unwrap_err()
            .cause,
        Cause::Shape
    );
    assert!(server.thread.join().unwrap()[0].contains("fields(first:$size,after:$after)"));
}

fn summary(number: u32) -> Value {
    json!({
        "id":format!("PVT_{number}"),"kind":"ProjectV2","number":number,
        "title":"Delivery","url":format!("https://github.com/orgs/PerishLab/projects/{number}"),
        "public":false,"updated":"2026-11-24T00:00:00Z",
        "owner":{"id":"O_perish","kind":"Organization","login":"PerishLab"},
    })
}

fn projects(rows: Vec<Value>, total: u32, next: Option<&str>) -> String {
    let list = json!({"total":total,"rows":rows,"page":{"more":next.is_some(),"cursor":next}});
    let organization = json!({
        "id":"O_perish","login":"PerishLab","kind":"Organization",
        "url":"https://github.com/PerishLab","list":list,
    });
    json!({"data":{"organization":organization}}).to_string()
}

#[test]
fn inventory() {
    let target = serve(vec![
        reply("200 OK", "", &projects(vec![summary(6)], 2, Some("next"))),
        reply("200 OK", "", &projects(vec![summary(1)], 2, None)),
    ]);
    let first = target
        .client
        .projects("perishlab", &Window::new(1, None).unwrap())
        .unwrap();
    assert_eq!(first.outcome, Outcome::Observed);
    assert_eq!(first.value.owner.id, first.value.projects.target);
    assert_eq!(first.value.projects.rows[0].number, 6);
    let window = Window::new(1, first.value.projects.next.as_deref()).unwrap();
    let second = target.client.projects("perishlab", &window).unwrap();
    assert_eq!(second.value.projects.rows[0].number, 1);
    assert!(second.value.projects.next.is_none());
    let requests = target.thread.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].contains("list:projectsV2(first:$size,after:$after)"));
    assert!(requests[1].contains(r#""after":"next""#));
}

#[test]
fn incomplete() {
    let mut foreign = summary(1);
    foreign["owner"]["id"] = json!("O_foreign");
    let target = serve(vec![
        reply("200 OK", "", &projects(vec![foreign], 1, None)),
        reply(
            "200 OK",
            "",
            &projects(vec![summary(1), summary(1)], 2, None),
        ),
        reply("200 OK", "", &projects(vec![], 1, None)),
        reply("200 OK", "", &projects(vec![summary(1)], 2, Some("same"))),
    ]);
    for after in [None, None, None, Some("same")] {
        let error = target
            .client
            .projects("PerishLab", &Window::new(2, after).unwrap())
            .unwrap_err();
        assert_eq!(error.cause, Cause::Shape);
    }
    assert_eq!(target.thread.join().unwrap().len(), 4);
}
