#[cfg(target_os = "macos")]
mod macos;

#[cfg(all(test, target_os = "macos"))]
pub(crate) use macos::set_test_pick_hook;
