mod commands;
mod dto;
mod error;
mod state;

use loldraft_data::Database;
use state::AppState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("No se pudo resolver el directorio de datos: {error}"))?;
            std::fs::create_dir_all(&data_dir).map_err(|error| {
                format!(
                    "No se pudo crear el directorio de datos {}: {error}",
                    data_dir.display()
                )
            })?;

            let db_path = data_dir.join("loldraft.sqlite3");
            let db = Database::open(&db_path).map_err(|error| {
                format!(
                    "No se pudo abrir la base de datos {}: {error}",
                    db_path.display()
                )
            })?;

            let state = AppState::new(db).map_err(|error| error.message)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_bootstrap,
            commands::champions_list,
            commands::data_status,
            commands::data_sync_ddragon,
            commands::profiles_list,
            commands::profiles_import_pilot,
            commands::profiles_import_proposal,
            commands::profiles_activate,
            commands::analysis_current,
            commands::profile_champion,
            commands::draft_new,
            commands::draft_action,
            commands::draft_undo,
            commands::draft_redo,
            commands::draft_set_role,
            commands::draft_list,
            commands::draft_load,
            commands::draft_delete,
            commands::settings_get,
            commands::settings_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running LolDraft");
}
