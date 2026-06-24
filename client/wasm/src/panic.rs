use std::panic;
use std::sync::Once;

use js_sys::{Function, Reflect};
use wasm_bindgen::{JsCast, JsValue};

static PANIC_HOOK: Once = Once::new();

/// Install the client panic reporter once.
pub(crate) fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        panic::set_hook(Box::new(report_panic));
    });
}

/// Report one panic through the JavaScript console.
fn report_panic(info: &panic::PanicHookInfo<'_>) {
    let message = info.to_string();
    let global = js_sys::global();
    let console = Reflect::get(&global, &JsValue::from_str("console")).ok();
    let error =
        console.and_then(|console| Reflect::get(&console, &JsValue::from_str("error")).ok());

    let Some(error) = error.and_then(|error| error.dyn_into::<Function>().ok()) else {
        return;
    };

    let _ = error.call1(&JsValue::UNDEFINED, &JsValue::from_str(&message));
}
