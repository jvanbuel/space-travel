mod app;
mod game;
mod tables;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
mod tui;
#[cfg(target_arch = "wasm32")]
mod web;

fn main() -> std::io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    return native::run();
    #[cfg(target_arch = "wasm32")]
    return web::run();
}
