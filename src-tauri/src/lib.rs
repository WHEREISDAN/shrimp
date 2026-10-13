//! Shrimp: speech to text, anywhere. See docs/ARCHITECTURE.md for the module map.

mod app;
pub mod audio;
mod commands;
pub mod dictation;
pub mod hotkeys;
pub mod llm;
pub mod notify;
pub mod platform;
pub mod sounds;
pub mod stt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Must be the first plugin: a second launch focuses this instance and exits.
    let builder =
        tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            app::window::show_main(app)
        }));

    #[cfg(debug_assertions)]
    let builder = builder.plugin(app::dev_bridge::plugin());
    let builder = platform::register_plugins(builder);

    macro_rules! handler {
        ($($dev:tt)*) => {
            tauri::generate_handler![
                commands::app::app_info,
                commands::dictation::get_dictation_state,
                commands::dictation::flow_bar_control,
                commands::dictation::flow_bar_ready,
                commands::dictation::flow_bar_layout,
                commands::dictation::run_notice_action,
                commands::dictation::get_shortcuts,
                commands::models::get_models,
                commands::models::download_default_model,
                commands::permissions::get_permissions,
                commands::permissions::open_permission_settings,
                $($dev)*
            ]
        };
    }
    #[cfg(debug_assertions)]
    let handler: fn(tauri::ipc::Invoke) -> bool = handler!(
        commands::dev::dev_get_audio_source,
        commands::dev::dev_set_audio_source
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
