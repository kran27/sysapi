use axum_server::Server;
use axum::{routing::get, Router, Json};
use serde::Serialize;
use sysinfo::{System, Disks};
use std::net::SocketAddr;
use std::sync::Mutex;
use lazy_static::lazy_static;

#[derive(Serialize)]
struct ServerStats {
    cpu_usage: String,
    ram: UsageInfo,
    storage: UsageInfo,
}

#[derive(Serialize)]
struct UsageInfo {
    used: String,
    total: String,
    percentage: String,
}

lazy_static! {
    static ref SYSTEM: Mutex<System> = Mutex::new(System::new_all());
    static ref DISKS: Mutex<Disks> = Mutex::new(Disks::new_with_refreshed_list());
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    match bytes {
        b if b >= TB => format!("{:.2} TB", b as f64 / TB as f64),
        b if b >= GB => format!("{:.2} GB", b as f64 / GB as f64),
        b if b >= MB => format!("{:.2} MB", b as f64 / MB as f64),
        b if b >= KB => format!("{:.2} KB", b as f64 / KB as f64),
        _ => format!("{} B", bytes),
    }
}

fn format_percentage(value: f32) -> String {
    format!("{:.2}%", value)
}

async fn get_server_stats() -> Json<ServerStats> {
    let mut system = SYSTEM.lock().unwrap();
    system.refresh_all();

    let cpu_usage = format_percentage(system.global_cpu_usage());

    let total_memory = system.total_memory();
    let used_memory = system.used_memory();
    let ram_percentage = (used_memory as f32 / total_memory as f32) * 100.0;

    let ram = UsageInfo {
        used: format_bytes(used_memory),
        total: format_bytes(total_memory),
        percentage: format_percentage(ram_percentage),
    };

    let mut disks = DISKS.lock().unwrap();
    disks.refresh_list();
    let total_disk_space = disks.iter().map(|disk| disk.total_space()).sum::<u64>();
    let available_disk_space = disks.iter().map(|disk| disk.available_space()).sum::<u64>();
    let used_percentage = 100.0 - (available_disk_space as f32 / total_disk_space as f32) * 100.0;

    let storage = UsageInfo {
        used: format_bytes(total_disk_space - available_disk_space),
        total: format_bytes(total_disk_space),
        percentage: format_percentage(used_percentage),
    };

    Json(ServerStats {
        cpu_usage,
        ram,
        storage,
    })
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/stats", get(get_server_stats));
    let addr = SocketAddr::from(([127, 0, 0, 1], 2989));
    println!("Listening on {}", addr);
    if let Err(e) = Server::bind(addr).serve(app.into_make_service()).await {
        eprintln!("Server error: {}", e);
    }
}