pub mod background;

#[allow(dead_code)]
#[cfg(any(test, target_os = "android"))]
pub mod bluetooth;

pub mod calendar;

#[allow(dead_code)]
#[cfg(any(test, target_os = "android"))]
pub mod camera;

pub mod contact;

pub mod core;

pub mod describe;

pub mod document;

pub mod intent;

pub mod location;

pub mod media;

#[allow(dead_code)]
#[cfg(any(test, target_os = "android"))]
pub mod midi;

pub mod notification;

pub mod permission;

pub mod text;

#[allow(dead_code)]
#[cfg(any(test, target_os = "android"))]
pub mod usb;
