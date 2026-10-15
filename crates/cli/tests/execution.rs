#[path = "execution/fixture.rs"]
mod fixture;

use fixture::{ENABLED, Seat, args};
#[cfg(unix)]
use serde_json::json;

#[test]
fn failure() {
    let home = Seat::new();
    home.configure(ENABLED);
    let command = args(&[":", "absent-private-program", "secret-token"]);
    let result = home.run(&command);
    assert!(!result.status.success());
    let events = home.facts();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["context"]["runseal.wrapper"], "opaque");
    let atoms = home.records();
    let finish = atoms
        .iter()
        .find(|atom| atom["payload"]["event"] == "cli.finish")
        .unwrap();
    assert_eq!(finish["payload"]["code"], 1);
    let text = serde_json::to_string(&atoms).unwrap();
    assert!(!text.contains("absent-private-program"));
    assert!(!text.contains("secret-token"));
}

#[test]
fn routing() {
    let home = Seat::new();
    home.configure(ENABLED);
    for command in [args(&["profile"]), args(&[":", "@missing"]), args(&[":"])] {
        home.run(&command);
    }
    assert!(home.facts().is_empty());
}

#[test]
#[cfg(unix)]
fn programs() {
    let home = Seat::programs();
    for program in ["gh", "concord", "plumb"] {
        home.compare(&args(&[
            ":",
            "env",
            "GODEBUG=http2client=0",
            program,
            "--version",
        ]));
    }
    let events = home.facts();
    assert_eq!(events.len(), 3);
    for (event, program) in events.iter().zip(["gh", "concord", "plumb"]) {
        assert_eq!(event["context"]["runseal.program"], "env");
        assert_eq!(event["context"]["runseal.target"], program);
        assert_eq!(event["context"]["runseal.wrapper"], "env");
        assert_eq!(
            event["payload"],
            json!([["GODEBUG=http2client=0"], "env", program, "--version"])
        );
    }
}

#[test]
#[cfg(unix)]
fn inherited() {
    let home = Seat::programs();
    home.compare(&args(&[":", "gh", "api", "private-path"]));
    let event = home.facts().pop().unwrap();
    assert_eq!(event["payload"], json!([[], "gh", "gh", "api"]));
    assert_eq!(event["context"]["runseal.wrapper"], "none");
    assert!(
        !serde_json::to_string(&home.records())
            .unwrap()
            .contains("inherited-private")
    );
}

#[test]
#[cfg(unix)]
fn rewritten() {
    let home = Seat::programs();
    std::fs::write(home.path().join("runseal.toml"), "[env.vars]\nPRIVATE_MARKER = 'profile-private'\n[argv]\nenv = ['GODEBUG=http2client=0', 'gh', 'api']\n").unwrap();
    let command = args(&[":", "env", "private-argument"]);
    home.compare(&command);
    assert_eq!(
        home.facts()[0]["payload"],
        json!([["GODEBUG=http2client=0"], "env", "gh", "api"])
    );
    let text = serde_json::to_string(&home.records()).unwrap();
    assert!(!text.contains("profile-private"));
    assert!(!text.contains("PRIVATE_MARKER"));
    assert!(!text.contains("private-argument"));
}

#[test]
#[cfg(unix)]
fn secrets() {
    let home = Seat::programs();
    let command = args(&[
        ":",
        "env",
        "GH_TOKEN=SECRET_TOKEN_35",
        "GODEBUG=PRIVATE_DEBUG_35",
        "gh",
        "api",
        "repos/private-path-35",
        "-H",
        "Authorization: PRIVATE_HEADER_35",
    ]);
    home.compare(&command);
    assert_eq!(
        home.facts()[0]["payload"],
        json!([["other", "other"], "env", "gh", "api"])
    );
    let text = serde_json::to_string(&home.records()).unwrap();
    for secret in [
        "GH_TOKEN",
        "SECRET_TOKEN_35",
        "PRIVATE_DEBUG_35",
        "private-path-35",
        "PRIVATE_HEADER_35",
        "Authorization",
        "bc18a8579d802fce8e5b08fff7c4dcd164f33cb69aa40d0eff1a2ecec10f2463",
        "e1f896774ccec251afc3b42ab402f25ad13da0710b919c3909fa8a86469170e2",
        "97d5fc01f1dd456a6495816a5f9cddd62ccb063a1f5ccbebccb437775b8de618",
        "fa6efb75e37af45ec33e67aae16ee796ad17b11a3c8b5331c62073e4ac7ac159",
    ] {
        assert!(!text.contains(secret));
    }
}

#[test]
#[cfg(unix)]
fn opaque() {
    let home = Seat::programs();
    let private = home.path().join("bin/private");
    let command = private.to_str().unwrap();
    for argv in [
        args(&[":", "env", "-i", "gh", "--version"]),
        args(&[":", command, "private-argument"]),
        args(&[":", "sh", "-c", "printf shell-private"]),
        args(&[":", "env", "env", "gh", "--version"]),
        args(&[":", "env", "invalid-name=private", "gh"]),
    ] {
        home.compare(&argv);
    }
    let events = home.facts();
    assert_eq!(events.len(), 5);
    assert!(
        events
            .iter()
            .all(|event| event["context"]["runseal.wrapper"] == "opaque")
    );
    let text = serde_json::to_string(&home.records()).unwrap();
    for secret in [command, "private-argument", "shell-private", "invalid-name"] {
        assert!(!text.contains(secret));
    }
}

#[test]
#[cfg(unix)]
fn budget() {
    let home = Seat::programs();
    let mut count = args(&[":", "gh"]);
    count.extend((0..32).map(|_| "private-count".to_string()));
    let long = vec![":".to_string(), "gh".to_string(), "私".repeat(43)];
    for argv in [count, long] {
        home.compare(&argv);
    }
    assert!(
        home.facts()
            .iter()
            .all(|event| event["payload"] == json!([[], "other", "other", "other"]))
    );
    let mut maximum = args(&[":", "env"]);
    maximum.extend((0..30).map(|_| "GODEBUG=http2client=0".to_string()));
    maximum.push("gh".to_string());
    home.compare(&maximum);
    let event = home.facts().pop().unwrap();
    assert_eq!(event["payload"][0].as_array().unwrap().len(), 30);
}

#[test]
#[cfg(unix)]
fn isolation() {
    let home = Seat::programs();
    let command = args(&[":", "gh", "api", "private-argument"]);
    home.configure("[locus]\nenabled = false\n");
    let baseline = home.run(&command);
    assert!(!home.path().join("state/locus").exists());
    for config in [
        ENABLED,
        "[locus]\nenabled = 'malformed'\n",
        "[locus]\nenabled = true\nendpoint = ''\n",
    ] {
        home.configure(config);
        let observed = home.run(&command);
        assert_eq!(baseline.status.code(), observed.status.code());
        assert_eq!(baseline.stdout, observed.stdout);
        assert!(observed.stderr.ends_with(&baseline.stderr));
    }
    assert_eq!(home.facts().len(), 1);
    let blocked = Seat::programs();
    std::fs::write(blocked.path().join("state"), "occupied").unwrap();
    blocked.compare(&command);
    assert_eq!(
        std::fs::read_to_string(blocked.path().join("state")).unwrap(),
        "occupied"
    );
}
