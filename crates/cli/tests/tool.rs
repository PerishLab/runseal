#[path = "support/cloudflare/mod.rs"]
mod cloudflare;
mod support;

use support::{Seat, text};

#[test]
fn unknown() {
    let seat = Seat::new();
    let output = seat.run(&[":", "@missing"]);
    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("unknown Runseal tool: @missing"));
}

#[test]
fn tencent() {
    let seat = Seat::new();
    let output = seat.run(&[":", "@tencent", "compute", "--help"]);
    assert!(output.status.success(), "{output:?}");
    let help = text(&output.stdout);
    for query in [
        "instances",
        "regions",
        "zones",
        "shapes",
        "images",
        "blueprints",
        "bundles",
    ] {
        assert!(help.contains(query), "{help}");
    }
    let output = seat.run(&[":", "@tencent", "compute", "instances", "--help"]);
    assert!(output.status.success(), "{output:?}");
    let help = text(&output.stdout);
    for argument in ["--product", "--region", "--offset", "--limit"] {
        assert!(help.contains(argument), "{help}");
    }
    let output = seat.run(&[":", "@tencent", "compute", "--action", "RunInstances"]);
    assert!(!output.status.success());
}
