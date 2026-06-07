use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_dir = app.path().app_data_dir().unwrap();
            println!("APP_DATA_DIR: {:?}", app_dir);
            println!("EXISTS: {}", app_dir.exists());
            let parent = app_dir.parent().unwrap();
            println!("PARENT: {:?}", parent);
            println!("PARENT_EXISTS: {}", parent.exists());
            std::process::exit(0);
        })
        .run(tauri::generate_context!())
        .unwrap();
}
