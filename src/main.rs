mod agent;
mod backend;
mod commands;
mod log;
mod record;
mod storage;

use backend::CoworkBackend;

fn main() {
    let backend = CoworkBackend::init().expect("failed to initialize Cowork backend");

    tauri::Builder::default()
        .manage(backend)
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap_app,
            commands::load_session,
            commands::new_session,
            commands::delete_session,
            commands::send_message,
            commands::get_settings,
            commands::save_provider_config,
            commands::save_mcp_servers_config,
            commands::reload_provider_config,
            commands::reload_mcp_servers_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Cowork RS");
}
