use super::*;

fn state() -> Value {
    let grant = json!({"credential":{"token":"USER_FIXTURE","expires":4102444800_i64},
        "refresh":"REFRESH_FIXTURE","expires":4102444800_i64,"scopes":["offline_access"]});
    json!({"version":2,"app":"cli_fixture","secret":"SECRET_FIXTURE",
        "credential":{"token":"BOT_FIXTURE","expires":4102444800_i64},
        "identity":{"app":"cli_fixture","bot":"ou_bot","observed":100,"stamp":format!("{:x}",Sha256::digest(b"BOT_FIXTURE"))},
        "user":{"expected":"ou_user","uncertain":false,"device":null,"grant":grant,
            "identity":{"app":"cli_fixture","bot":"","user":"ou_user","observed":100,"stamp":format!("{:x}",Sha256::digest(b"USER_FIXTURE"))}}})
}

#[test]
fn isolation() {
    let fixture = Fixture::new();
    fixture.write(state());
    assert!(Client::user(&fixture.state, "cli_fixture", "ou_other").is_err());
    let mut user = Client::user(&fixture.state, "cli_fixture", "ou_user").unwrap();
    let status = user.status();
    assert!(status.valid);
    assert_eq!(status.identity.unwrap().user.as_deref(), Some("ou_user"));
    assert!(
        !serde_json::to_string(&user.status())
            .unwrap()
            .contains("FIXTURE")
    );
    user.logout().unwrap();
    assert!(!user.status().valid);
    drop(user);
    let bot = Client::open(&fixture.state, "cli_fixture").unwrap();
    assert_eq!(bot.status().identity.unwrap().bot, "ou_bot");
    assert!(bot.status().valid);
}

#[test]
fn uncertain() {
    let fixture = Fixture::new();
    let mut value = state();
    value["user"]["uncertain"] = json!(true);
    fixture.write(value);
    let mut user = Client::user(&fixture.state, "cli_fixture", "ou_user").unwrap();
    assert!(!user.status().valid);
    assert!(user.renew().is_err());
    assert!(user.exchange().is_err());
    assert!(user.identity().is_err());
    assert!(user.begin(&["offline_access".into()]).is_err());
    let error = user
        .message(&Operation::Get {
            message: "om_fixture".into(),
        })
        .unwrap_err();
    assert_eq!(error.outcome, Outcome::Unsent);
    user.logout().unwrap();
    assert!(!user.status().valid);
}

#[test]
fn refusal() {
    for mutation in ["stamp", "app", "bot", "user"] {
        let fixture = Fixture::new();
        let mut value = state();
        value["user"]["identity"][mutation] = json!("foreign");
        fixture.write(value);
        let user = Client::user(&fixture.state, "cli_fixture", "ou_user").unwrap();
        let error = user
            .message(&Operation::Get {
                message: "om_fixture".into(),
            })
            .unwrap_err();
        assert_eq!(error.class, Class::Binding);
        assert_eq!(error.outcome, Outcome::Unsent);
    }
    let fixture = Fixture::new();
    let mut value = state();
    value["version"] = json!(1);
    fixture.write(value);
    assert!(Client::open(&fixture.state, "cli_fixture").is_err());
}

#[test]
fn scope() {
    let fixture = Fixture::new();
    let mut value = state();
    value["user"] = Value::Null;
    fixture.write(value);
    let mut user = Client::user(&fixture.state, "cli_fixture", "ou_user").unwrap();
    assert!(user.begin(&["im:message.send_as_user".into()]).is_err());
    assert!(user.begin(&["offline_access BAD".into()]).is_err());
    assert!(user.status().user.is_none());
}

#[test]
fn profile() {
    let fixture = Fixture::new();
    fixture.write(state());
    let vars = BTreeMap::from([
        (
            "FEISHU_STATE_FILE".into(),
            fixture.state.display().to_string(),
        ),
        ("FEISHU_APP_ID".into(), "cli_fixture".into()),
        ("FEISHU_ACTOR".into(), "user".into()),
        ("FEISHU_USER_OPEN_ID".into(), "ou_user".into()),
    ]);
    let words = |args: &[&str]| args.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let output = tool::call("feishu", &words(&["auth", "status"]), &vars).unwrap();
    assert_eq!(output.value["actor"], "user");
    assert_eq!(output.value["expected"], "ou_user");
    assert_eq!(output.value["identity"]["user"], "ou_user");
    let output = tool::call("feishu", &words(&["--as", "bot", "auth", "status"]), &vars).unwrap();
    assert_eq!(output.value["actor"], "bot");
    assert_eq!(output.value["identity"]["bot"], "ou_bot");
}

#[test]
fn recovery() {
    let fixture = Fixture::new();
    let mut value = state();
    value["version"] = json!(3);
    value["user"]["uncertain"] = json!(true);
    value["user"]["returned"] = json!({"kind":"Exchange","started":100,"request":"SAFE-REQUEST",
        "body":json!({"token_type":"Bearer","access_token":"RETURNED_SECRET","expires_in":7200,
            "refresh_token":"REFRESH_SECRET","refresh_token_expires_in":604800,"scope":"offline_access"}).to_string()});
    fixture.write(value);
    let mut client = Client::user(&fixture.state, "cli_fixture", "ou_user").unwrap();
    assert!(client.status().user.unwrap().retained);
    let status = client.recover().unwrap();
    assert_eq!(status.expires, Some(7300));
    assert!(!status.valid);
    assert!(status.identity.is_none());
    let user = status.user.unwrap();
    assert!(!user.retained && !user.uncertain);
    assert_eq!(user.renewable, Some(604900));
    drop(client);
    let bot = Client::open(&fixture.state, "cli_fixture").unwrap();
    assert!(bot.status().valid);
    assert_eq!(bot.status().identity.unwrap().bot, "ou_bot");
    drop(bot);
    let output = fixture.call(&["--as", "user", "--user", "ou_user", "auth", "recover"]);
    assert!(output.is_err());
}

#[test]
fn preservation() {
    for version in [2, 3] {
        let fixture = Fixture::new();
        let mut value = state();
        value["version"] = json!(version);
        value["user"]["uncertain"] = json!(true);
        value["user"]["returned"] = json!({"kind":"Exchange","started":100,"request":null,
            "body":json!({"token_type":"DPoP","access_token":"SECRET_FIXTURE","expires_in":7200}).to_string()});
        fixture.write(value);
        let opened = Client::user(&fixture.state, "cli_fixture", "ou_user");
        if version == 2 {
            assert!(opened.is_err());
            continue;
        }
        let mut client = opened.unwrap();
        let error = client.recover().unwrap_err();
        assert_eq!(error.field, Some(runseal::feishu::Field::Kind));
        assert!(!format!("{error:?} {error}").contains("SECRET"));
        let status = client.status();
        assert!(status.user.unwrap().retained);
        assert!(
            !serde_json::to_string(&client.status())
                .unwrap()
                .contains("SECRET")
        );
        assert_eq!(client.exchange().unwrap_err().outcome, Outcome::Unsent);
    }
}
