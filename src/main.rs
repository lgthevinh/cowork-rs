mod agent;
mod app;
mod log;
mod record;
mod storage;

use agent::agent_orchestrator;
use storage::chat_record;

fn main() -> iced::Result {
    // init record storage
    let db = record::SqliteDb::open("data.db").expect("failed to open sqlite database");
    db.init_record::<chat_record::SessionRecord>()
        .expect("failed to initialize session schema");
    db.init_record::<chat_record::MessageRecord>()
        .expect("failed to initialize message schema");

    // init agent orchestrator
    let agent_orchestrator: agent_orchestrator::AgentOrchestrator =
        agent_orchestrator::init().expect("failed to initialize agent orchestrator");

    // init ui app
    app::run(db, agent_orchestrator)
}
