use napi_derive::napi;

// nocheckin: proper NAPI binding

#[napi]
pub fn fibonacci(n: u32) -> u32 {
    match n {
        1 | 2 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
