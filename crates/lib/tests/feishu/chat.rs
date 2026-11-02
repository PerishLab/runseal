use super::*;
use runseal::feishu::chat::Get;

#[test]
fn binding() {
    let fixture = Fixture::new();
    fixture.write(json!({"version":1,"app":"cli_fixture","secret":"SECRET_FIXTURE","credential":null,"identity":null}));
    let client = Client::open(&fixture.state, "cli_fixture").unwrap();
    let failure = client
        .chat(&Get {
            chat: "oc_fixture".into(),
        })
        .unwrap_err();
    assert_eq!(failure.class, Class::Authentication);
    assert_eq!(failure.outcome, Outcome::Unsent);
}

#[test]
fn admission() {
    let fixture = Fixture::new();
    fixture.write(json!({"version":1,"app":"cli_fixture","secret":"SECRET_FIXTURE","credential":null,"identity":null}));
    let _held = Client::open(&fixture.state, "cli_fixture").unwrap();
    let failure = fixture
        .call(&["im", "chats", "get", "--chat-id", "oc_unsafe?x=y"])
        .unwrap_err();
    assert_eq!(
        failure
            .downcast_ref::<runseal::feishu::Error>()
            .unwrap()
            .class,
        Class::Input
    );
}
