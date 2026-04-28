mod types;
mod board;
mod movegen;
mod eval;
mod tt;
mod search;
#[cfg(not(target_arch = "wasm32"))]
mod uci;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    uci::run();
}
