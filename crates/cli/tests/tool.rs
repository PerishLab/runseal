#[path = "support/cloudflare/mod.rs"]
mod cloudflare;
#[path = "support/feishu.rs"]
mod feishu;
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

#[test]
fn dns() {
    let seat = Seat::new();
    let output = seat.run(&[":", "@tencent", "dns", "record", "get", "--help"]);
    assert!(output.status.success(), "{output:?}");
    for argument in ["--domain", "--domain-id", "--id"] {
        assert!(text(&output.stdout).contains(argument));
    }
    let output = seat.run(&[
        ":",
        "@tencent",
        "dns",
        "domain",
        "--domain",
        "example.org",
        "--domain-id",
        "42",
    ]);
    assert!(!output.status.success());
    let output = seat.run(&[
        ":",
        "@tencent",
        "dns",
        "record",
        "get",
        "--domain-id",
        "42",
        "--id",
        "9",
        "--raw-body",
        "{}",
    ]);
    assert!(!output.status.success());
}

#[test]
fn cvm() {
    let seat = Seat::new();
    let output = seat.run(&[":", "@tencent", "cvm", "run", "--help"]);
    assert!(output.status.success(), "{output:?}");
    for argument in [
        "--client-token",
        "--dry-run",
        "--group",
        "--key",
        "--disk-class",
    ] {
        assert!(text(&output.stdout).contains(argument));
    }
    let output = seat.run(&[":", "@tencent", "cvm", "key", "import", "--help"]);
    assert!(output.status.success(), "{output:?}");
    assert!(text(&output.stdout).contains("--public-file"));
    let output = seat.run(&[
        ":",
        "@tencent",
        "cvm",
        "--region",
        "ap-hongkong",
        "status",
        "--id",
        "ins-fixture",
        "--raw-body",
        "{}",
    ]);
    assert!(!output.status.success());
}

#[test]
fn interfaces() {
    let seat = Seat::new();
    let output = seat.run(&[":", "@tencent", "network", "interfaces", "--help"]);
    assert!(output.status.success(), "{output:?}");
    for flag in ["--instance", "--offset", "--limit"] {
        assert!(text(&output.stdout).contains(flag));
    }
    let output = seat.run(&[
        ":",
        "@tencent",
        "network",
        "--region",
        "ap-hongkong",
        "interfaces",
        "--instance",
        "ins-fixture",
        "--raw-body",
        "WITHHELD_FIXTURE",
    ]);
    assert!(!output.status.success());
    assert!(!text(&output.stderr).contains("WITHHELD_FIXTURE"));
}
