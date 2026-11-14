mod backend;
mod local;
mod serve;
mod tools;

use crate::app::Hub;
pub use backend::Suite;
use agent_ui::Agent;
use std::sync::Arc;

pub type Page = Agent<Hub>;

pub fn new(ctx: &egui::Context, open: bool) -> Page {
    Agent::new(ctx, Arc::new(Suite::default()), open)
}

pub fn run_helper() -> Option<i32> {
    agent_core::sandbox::run_helper()
}
