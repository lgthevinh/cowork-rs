mod agent;
mod app;
mod repo;

use agent::agent_orchestrator;
use repo::record::record_impl;

fn main() -> iced::Result {
    // init repo
    let db = repo::SqliteDb::open("data.db").expect("failed to open sqlite database");
    db.init_record::<record_impl::SessionRecord>()
        .expect("failed to initialize database schema");

    // init agent orchestrator
    agent_orchestrator::init();

    // init ui app
    app::run()
}
