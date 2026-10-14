//! Shrimp: speech to text, anywhere. See docs/ARCHITECTURE.md for the module map.

mod app;
pub mod audio;
mod commands;
pub mod context;
pub mod dictation;
pub mod edit;
pub mod health;
pub mod history;
pub mod hotkeys;
pub mod i18n;
pub mod insights;
pub mod lang;
pub mod llm;
pub mod mic;
pub mod notify;
pub mod platform;
pub mod secrets;
pub mod settings;
pub mod sounds;
pub mod storage;
pub mod stt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Must be the first plugin: a second launch focuses this instance and exits.
    // In debug builds, `--quit` asks the running instance to quit cleanly
    // (the Windows dev script uses it; scripts/macos/dev-app.sh quits through AppleScript).
    let builder =
        tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if cfg!(debug_assertions) && args.iter().any(|arg| arg == "--quit") {
                app.exit(0);
                return;
            }
            app::window::show_main(app)
        }));

    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    #[cfg(debug_assertions)]
    let builder = builder.plugin(app::dev_bridge::plugin());
    let builder = platform::register_plugins(builder);

    macro_rules! handler {
        ($($dev:tt)*) => {
            tauri::generate_handler![
                commands::app::app_info,
                commands::app::own_field_focused,
                commands::dictation::get_dictation_state,
                commands::dictation::flow_bar_control,
                commands::dictation::flow_bar_ready,
                commands::dictation::flow_bar_layout,
                commands::dictation::flow_bar_drag,
                commands::dictation::flow_bar_action,
                commands::dictation::run_notice_action,
                commands::dictation::transform_action,
                commands::dictation::flow_bar_wand,
                commands::shortcuts::get_shortcuts,
                commands::shortcuts::record_shortcut,
                commands::shortcuts::cancel_shortcut_recording,
                commands::shortcuts::remove_shortcut,
                commands::shortcuts::reset_shortcuts,
                commands::languages::get_languages,
                commands::languages::set_languages,
                commands::languages::set_active_language,
                commands::microphones::get_microphones,
                commands::microphones::select_microphone,
                commands::microphones::rank_microphone,
                commands::microphones::forget_microphone,
                commands::microphones::start_mic_preview,
                commands::microphones::stop_mic_preview,
                commands::models::get_models,
                commands::models::retry_model,
                commands::models::download_model,
                commands::models::delete_model,
                commands::permissions::get_permissions,
                commands::permissions::open_permission_settings,
                commands::settings::get_settings,
                commands::settings::update_settings,
                commands::settings::get_format_apps,
                commands::ai::get_ai_options,
                commands::ai::get_api_keys,
                commands::ai::set_api_key,
                commands::ai::delete_api_key,
                commands::ai::list_ollama_models,
                commands::ai::test_ai_provider,
                commands::library::list_dictionary,
                commands::library::add_dictionary_entry,
                commands::library::update_dictionary_entry,
                commands::library::set_dictionary_pinned,
                commands::library::delete_dictionary_entries,
                commands::library::preview_dictionary_import,
                commands::library::import_dictionary,
                commands::library::list_snippets,
                commands::library::add_snippet,
                commands::library::update_snippet,
                commands::library::delete_snippets,
                commands::library::preview_snippet_import,
                commands::library::import_snippets,
                commands::history::list_history,
                commands::history::copy_history,
                commands::history::set_history_raw,
                commands::history::retry_history,
                commands::history::history_audio,
                commands::history::extract_history_audio,
                commands::history::delete_history,
                commands::history::delete_all_history,
                commands::history::set_history_storage,
                commands::insights::get_insights,
                commands::insights::get_voice_profile,
                commands::insights::generate_voice_profile,
                commands::insights::copy_image,
                commands::insights::save_image,
                commands::system::get_app_health,
                commands::system::dismiss_crash,
                commands::system::check_storage,
                commands::system::open_storage_settings,
                commands::system::repair_data,
                commands::system::reset_app,
                commands::system::move_to_applications,
                commands::system::quit_app,
                $($dev)*
            ]
        };
    }
    #[cfg(debug_assertions)]
    let handler: fn(tauri::ipc::Invoke) -> bool = handler!(
        commands::dev::dev_get_audio_source,
        commands::dev::dev_set_audio_source,
        commands::dev::dev_set_app_category,
        commands::dev::dev_edit_text,
        commands::dev::dev_read_context,
        commands::dev::dev_set_microphones,
        commands::dev::dev_set_permissions
    );
    #[cfg(not(debug_assertions))]
    let handler: fn(tauri::ipc::Invoke) -> bool = handler!();

    builder
        .invoke_handler(handler)
        .setup(app::setup::setup)
        .on_window_event(app::window::on_window_event)
        .build(tauri::generate_context!())
        .expect("failed to build the Shrimp app")
        .run(app::on_run_event);
}
