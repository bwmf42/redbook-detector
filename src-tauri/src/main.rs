#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            redbook_detector::download_xhs,
            redbook_detector::open_in_finder,
            redbook_detector::select_folder,
        ])
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}
