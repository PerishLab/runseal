mod api;
mod auth;
mod command;
mod fault;
mod identity;
mod model;
mod oauth;
mod secret;

pub use auth::Auth;
pub use command::{call, run};
pub use fault::{Class, Error, Failure, Outcome};
pub use identity::Client;
pub use model::Identity;

#[cfg(test)]
mod tests;
