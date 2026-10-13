//! Shrimp: speech to text, anywhere. See docs/ARCHITECTURE.md for the module map.

mod app;
pub mod audio;
mod commands;
pub mod llm;
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

    builder
        .invoke_handler(tauri::generate_handler![commands::app::app_info])
        .setup(|app| {
            app::tray::create(app.handle())?;
            Ok(())
        })
        .on_window_event(app::window::on_window_event)
        .build(tauri::generate_context!())
        .expect("failed to build the Shrimp app")
        .run(app::on_run_event);
}
