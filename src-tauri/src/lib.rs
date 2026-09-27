//! ALXS-RL-Mod backend. See `docs/ARCHITECTURE.md` for the module map and
//! the IPC contract.

pub mod base;
pub mod game;

// engine slice
pub mod ball;
pub mod decals;
pub mod palette;
pub mod upk;

// catalog slice
pub mod catalog;
pub mod integrity;
pub mod presets;
pub mod swap;

// play slice
pub mod extras;
pub mod maps;
pub mod stats;

use std::sync::RwLock;

use tauri::Manager;

use crate::base::config::{AppConfig, ConfigState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _log_guard = base::logging::init();
    base::logs::prune_old();
    let config = AppConfig::load().unwrap_or_else(|err| {
        tracing::error!(%err, "config unreadable, using defaults");
        AppConfig::default()
    });

    // `--restore-stock` (run by the uninstaller): put the game back to stock
    // and exit, without a window and without the single-instance handoff.
    let restore_mode = std::env::args().any(|a| a == "--restore-stock");

    let mut builder = tauri::Builder::default();
    if !restore_mode {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }));
    }
    // Release builds only: dev builds have no signed update to install.
    #[cfg(not(debug_assertions))]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .manage(ConfigState(RwLock::new(config)))
        .setup(move |app| {
            let handle = app.handle();
            if restore_mode {
                let handle = handle.clone();
                std::thread::spawn(move || {
                    let code = match game::writer::restore_everything(&handle) {
                        Ok(n) => {
                            tracing::info!(files = n, "game restored to stock (uninstall)");
                            0
                        }
                        Err(err) => {
                            tracing::error!(%err, "restore on uninstall failed");
                            1
                        }
                    };
                    handle.exit(code);
                });
                return Ok(());
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }
            // Order matters: the game watcher and key ring first, integrity
            // last (it inspects what the other modules restored).
            type Init = fn(&tauri::AppHandle) -> base::AppResult<()>;
            let inits: [(&str, Init); 12] = [
                ("game", game::init),
                ("upk", upk::init),
                ("catalog", catalog::init),
                ("swap", swap::init),
                ("presets", presets::init),
                ("palette", palette::init),
                ("decals", decals::init),
                ("ball", ball::init),
                ("maps", maps::init),
                ("stats", stats::init),
                ("extras", extras::init),
                ("integrity", integrity::init),
            ];
            for (name, init) in inits {
                if let Err(err) = init(handle) {
                    // A failing feature must not take the whole app down.
                    tracing::error!(module = name, %err, "module init failed");
                }
            }
            tracing::info!(version = env!("CARGO_PKG_VERSION"), "ALXS-RL-Mod started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // core / game
            base::app::app_info,
            base::app::diagnostics_export,
            base::logs::logs_read,
            base::logs::logs_clean,
            base::config::config_get,
            base::config::config_update,
            game::game_status,
            game::game_set_install_dir,
            // engine: keys, thumbnails, palette, decals
            upk::commands::keys_status,
            upk::commands::keys_import,
            upk::commands::thumbnail_get,
            palette::commands::palette_status,
            palette::commands::palette_stock,
            palette::commands::palette_list,
            palette::commands::palette_save,
            palette::commands::palette_delete,
            palette::commands::palette_apply,
            palette::commands::palette_restore,
            decals::commands::decals_library,
            decals::commands::decals_library_folders_set,
            decals::commands::decals_status,
            decals::commands::decals_apply,
            decals::commands::decals_remove,
            decals::commands::decals_import_alphaconsole,
            ball::commands::ball_library,
            ball::commands::ball_status,
            ball::commands::ball_apply,
            ball::commands::ball_remove,
            ball::commands::ball_import_alphaconsole,
            // catalog: catalog, swaps, presets, integrity
            catalog::commands::catalog_get,
            catalog::commands::catalog_refresh,
            swap::commands::swap_list,
            swap::commands::swap_apply,
            swap::commands::swap_restore,
            swap::commands::swap_restore_all,
            swap::commands::swap_history,
            presets::commands::presets_list,
            presets::commands::presets_save,
            presets::commands::presets_delete,
            presets::commands::presets_apply,
            presets::commands::presets_capture,
            presets::commands::presets_export_code,
            presets::commands::presets_import_code,
            presets::commands::presets_random,
            presets::commands::presets_active,
            presets::commands::presets_update_from_current,
            integrity::commands::integrity_check,
            integrity::commands::integrity_reapply,
            integrity::commands::integrity_restore_stock,
            // play: maps, launch, stats, extras
            maps::commands::maps_list,
            maps::commands::maps_import,
            maps::commands::maps_delete,
            maps::commands::maps_update,
            maps::commands::maps_browse,
            maps::commands::maps_download,
            maps::commands::maps_activate,
            maps::commands::maps_deactivate,
            maps::commands::maps_session,
            maps::commands::maps_play_offline,
            extras::commands::launch_game,
            extras::commands::replays_list,
            extras::commands::bakkesmod_status,
            stats::commands::stats_status,
            stats::commands::stats_enable,
            stats::commands::stats_disable,
            stats::commands::tracker_session,
            stats::commands::tracker_reset,
            stats::commands::mmr_lookup,
            stats::commands::overlay_open,
            stats::commands::overlay_close,
        ])
        // Closing the main window quits the app: the hidden tracker.gg
        // browser and the overlay are helper windows and must not keep a
        // windowless process alive (a new launch would be handed to it and
        // show nothing).
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                window.app_handle().exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running ALXS-RL-Mod");
}
