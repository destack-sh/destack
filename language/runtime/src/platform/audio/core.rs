#![allow(dead_code)]
#![allow(unused_imports)]

pub(crate) use std::collections::{HashMap, HashSet, VecDeque};
pub(crate) use std::sync::{Arc, Condvar, Mutex, OnceLock};
pub(crate) use std::thread;
pub(crate) use std::thread::JoinHandle;
pub(crate) use std::time::{Duration, Instant};

pub(crate) use crate::diagnostic::{RuntimeError, RuntimeResult};
pub(crate) use crate::platform::diagnostic::PlatformErrorCode;
pub(crate) use crate::platform::resource::ResourceKind;
use crate::platform::{PlatformError, resource};
pub(crate) use crate::runtime::{BindingCallContext, NativeStringRef};

pub(crate) use super::{
    AudioBackend, AudioBackendCapabilityFlags, AudioBackendSelectionPolicy, AudioChannelLayout,
    AudioClockDomain, AudioClockQuality, AudioClockSnapshot, AudioDeviceCapabilityFlags,
    AudioDeviceDescriptor, AudioDeviceDirection, AudioDeviceListFlags, AudioDeviceListRequest,
    AudioDeviceOpenFlags, AudioDeviceOpenOptions, AudioEvent, AudioEventDeliveryMode,
    AudioEventKind, AudioEventOverflowPolicy, AudioEventSource, AudioEventSubscriptionFlags,
    AudioEventSubscriptionOptions, AudioSampleFormat, AudioShareMode, AudioStreamAvailability,
    AudioStreamClockDomain, AudioStreamConfig, AudioStreamDescriptor, AudioStreamFlags,
    AudioStreamOpenOptions, AudioStreamRequirementFlags, AudioStreamState, AudioStreamStateKind,
    AudioStreamStatusFlags, AudioStreamTiming, AudioStreamTransferMode,
    AudioSupportedEventSubscriptionFlags, AudioSupportedStreamClockDomains,
    AudioSupportedStreamFlags, AudioSupportedStreamRequirementFlags,
};

/// Compatibility alias for backend-specific stream open flags.
pub(crate) type AudioBackendOpenFlags = AudioDeviceOpenFlags;

mod clock;
mod codec;
mod constants;
mod device;
mod error;
mod event;
mod model;
mod stream;

pub(crate) use clock::*;
pub(crate) use codec::*;
pub(crate) use constants::*;
pub(crate) use device::*;
pub(crate) use error::*;
pub(crate) use event::*;
pub(crate) use model::*;
pub(crate) use stream::*;
