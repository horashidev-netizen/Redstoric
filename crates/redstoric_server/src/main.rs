//! Server chuyên dụng của Redstoric (không cửa sổ, không GPU).
//!
//! Chạy: `cargo run --bin redstoric_server -- --port 6000 --data server_data`
//! Client kết nối: `cargo run --bin Redstoric -- --connect <ip>:6000 --name Tam`

use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use redstoric_protocol::DEFAULT_PORT;
use redstoric_server::NetServerPlugin;

fn main() {
    let mut port = DEFAULT_PORT;
    let mut data_dir = std::path::PathBuf::from("server_data");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--port" {
            match args.next().and_then(|v| v.parse().ok()) {
                Some(p) => port = p,
                None => eprintln!("--port cần một số hợp lệ, dùng mặc định {DEFAULT_PORT}"),
            }
        } else if arg == "--data" {
            match args.next() {
                Some(dir) => data_dir = dir.into(),
                None => eprintln!("--data cần một đường dẫn thư mục, dùng mặc định server_data"),
            }
        }
    }

    App::new()
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 60.0))))
        .add_plugins(LogPlugin::default())
        .add_plugins(NetServerPlugin { port, data_dir })
        .run();
}
