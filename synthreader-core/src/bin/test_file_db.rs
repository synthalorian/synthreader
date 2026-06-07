use std::path::Path;

#[tokio::main]
async fn main() {
    let path = Path::new("/home/synth/.local/share/io.synthalorian.synthreader/library_test.db");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    
    // Test different URL formats
    let urls = vec![
        format!("sqlite:{}", path.display()),
        format!("sqlite://{}", path.display()),
        format!("sqlite:///{}?mode=rwc", path.display()),
    ];
    
    for url in urls {
        println!("\nTrying URL: {}", url);
        match sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await {
            Ok(pool) => {
                println!("  SUCCESS");
                match sqlx::query("SELECT 1").fetch_one(&pool).await {
                    Ok(_) => println!("  Query OK"),
                    Err(e) => println!("  Query failed: {}", e),
                }
            }
            Err(e) => println!("  FAILED: {}", e),
        }
    }
    
    let _ = std::fs::remove_file(path);
}
