pub mod diagnostic {
    pub use destack_runtime::diagnostic::RuntimeStatus;
}

pub mod host {
    pub mod abi {
        pub use destack_runtime::host::abi::*;
    }

    #[cfg(any(test, target_os = "android"))]
    pub mod android {
        pub mod abi {
            pub use destack_runtime::host::android::abi::*;
        }
    }

    #[cfg(any(test, target_os = "ios"))]
    pub mod apple {
        pub mod abi {
            pub use destack_runtime::host::apple::abi::*;
        }
    }
}

pub mod platform {
    pub mod abi {
        pub use destack_runtime::platform::abi::{NativeSlice, NativeStringRef, NativeStringSlice};
    }

    pub mod os {
        pub use destack_runtime::platform::os::{
            DocumentDescriptorValue, LocationSample, LocationSampleValue, LocationWatchOptions,
        };
    }
}

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
pub use android::*;

#[cfg(any(test, target_os = "ios"))]
mod ios;
#[cfg(any(test, target_os = "ios"))]
pub use ios::*;
