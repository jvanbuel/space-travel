mod app;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

fn main() -> std::io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    return native::run();
    #[cfg(target_arch = "wasm32")]
    return web::run();
}
