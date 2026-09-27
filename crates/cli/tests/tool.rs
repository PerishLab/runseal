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
