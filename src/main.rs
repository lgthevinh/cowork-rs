mod agent;
mod app;
mod repo;

use agent::agent_orchestrator;
use repo::record::record_impl;

fn main() -> iced::Result {
    // init repo
    let db = repo::SqliteDb::open("data.db").expect("failed to open sqlite database");
    db.init_record::<record_impl::SessionRecord>()
        .expect("failed to initialize session schema");
    db.init_record::<record_impl::MessageRecord>()
        .expect("failed to initialize message schema");

    // init agent orchestrator
    let _agent_orchestrator =
        agent_orchestrator::init().expect("failed to initialize agent orchestrator");

    // init ui app
    app::run()
}
