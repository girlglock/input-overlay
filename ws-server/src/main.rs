// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod services;
mod single_instance;
mod system;
mod tray;
mod ui;
mod views;
mod ws_server;

fn main() {
    app::run()
}
