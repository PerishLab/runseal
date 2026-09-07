pub use plumb::skill::command::Deed;
use plumb::skill::{Kit, command::Command};
use std::path::PathBuf;

const RELEASES: &str = "https://releases.runseal.perish.uk";
const DEPOT: &str = "https://depot.runseal.perish.uk";

#[derive(Debug, PartialEq)]
struct Rig {
    home: String,
    releases: String,
}

impl Default for Rig {
    fn default() -> Self {
        Self {
            home: plumb::config::data("runseal")
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            releases: RELEASES.to_string(),
        }
    }
}

impl Rig {
    fn resolve() -> Self {
        let default = Self::default();
        Self {
            home: plumb::config::value("RUNSEAL_HOME").unwrap_or(default.home),
            releases: plumb::config::value("RUNSEAL_RELEASES").unwrap_or(default.releases),
        }
    }
}

pub fn run(deed: Deed) -> i32 {
    let rig = Rig::resolve();
    if rig.home.is_empty() {
        eprintln!("runseal skill: no data home; set RUNSEAL_HOME");
        return 1;
    }
    let kit = Kit {
        name: "runseal".to_string(),
        home: plumb::config::home().unwrap_or_else(|| PathBuf::from(".")),
        state: PathBuf::from(&rig.home).join("state").join("skills.json"),
        url: rig.releases,
    };
    let depot = kit.depot(DEPOT, "runseal", plumb::version!("RUNSEAL"));
    Command("runseal").run(&depot, deed)
}
