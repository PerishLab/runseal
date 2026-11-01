use super::*;
use runseal::feishu::reaction::{Create, Page};

#[test]
fn binding() {
    let fixture = Fixture::new();
    fixture.write(json!({"version":1,"app":"cli_fixture","secret":"SECRET_FIXTURE","credential":null,"identity":null}));
    let client = Client::open(&fixture.state, "cli_fixture").unwrap();
    let create = Create {
        message: "om_fixture".into(),
        emoji: "Get".into(),
    };
    let error = client.reaction(&create).unwrap_err();
    assert_eq!(error.class, Class::Authentication);
    assert_eq!(error.outcome, Outcome::Unsent);
    let page = Page {
        message: "om_fixture".into(),
        emoji: Some("Get".into()),
        size: 20,
        cursor: None,
    };
    assert_eq!(
        client.reactions(&page).unwrap_err().outcome,
        Outcome::Unsent
    );
}

#[test]
fn admission() {
    let fixture = Fixture::new();
    fixture.write(json!({"version":1,"app":"cli_fixture","secret":"SECRET_FIXTURE","credential":null,"identity":null}));
    let _held = Client::open(&fixture.state, "cli_fixture").unwrap();
    for words in [
        vec![
            "im",
            "reactions",
            "create",
            "--message-id",
            "om_fixture",
            "--emoji-type",
            "bad&emoji",
        ],
        vec![
            "im",
            "reactions",
            "list",
            "--message-id",
            "om_fixture",
            "--page-size",
            "51",
        ],
    ] {
        let error = fixture.call(&words).unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<runseal::feishu::Error>()
                .unwrap()
                .class,
            Class::Input
        );
    }
}
