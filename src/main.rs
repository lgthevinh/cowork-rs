mod agent;
mod app;

use agent::agent_orchestrator;

fn main() -> iced::Result {
    agent_orchestrator::init();
    app::run()
}
