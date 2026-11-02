use super::*;
use runseal::feishu::Search;

#[test]
fn binding() {
    let fixture = Fixture::new();
    fixture.write(json!({"version":1,"app":"cli_fixture","secret":"SECRET_FIXTURE","credential":null,"identity":null}));
    let client = Client::open(&fixture.state, "cli_fixture").unwrap();
    let search = Search {
        query: String::new(),
        start: 1,
        end: 2,
        size: 20,
        cursor: None,
    };
    let error = client.search(&search).unwrap_err();
    assert_eq!(error.class, Class::Authentication);
    assert_eq!(error.outcome, Outcome::Unsent);
}

#[test]
fn admission() {
    let fixture = Fixture::new();
    fixture.write(json!({"version":1,"app":"cli_fixture","secret":"SECRET_FIXTURE","credential":null,"identity":null}));
    let _held = Client::open(&fixture.state, "cli_fixture").unwrap();
    for (start, end, size) in [("2", "1", "20"), ("1", "2", "51")] {
        let error = fixture
            .call(&[
                "im",
                "messages",
                "search",
                "--start-time",
                start,
                "--end-time",
                end,
                "--page-size",
                size,
            ])
            .unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<runseal::feishu::Error>()
                .unwrap()
                .class,
            Class::Input
        );
    }
}
