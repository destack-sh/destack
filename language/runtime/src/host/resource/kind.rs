use serde::{Deserialize, Serialize};

/// Enumerate the canonical handle-to-kind registry entries.
macro_rules! for_each_resource_handle_kind {
    ($macro:ident) => {
        $macro! {
            (FileHandle, File, "host.fs.file", "file", "The handle for an open file."),
            (DirectoryHandle, Directory, "host.fs.directory", "directory", "The handle for an open directory."),
            (SocketHandle, Socket, "host.net.socket", "socket", "The handle for a network socket."),
            (ListenerHandle, Listener, "host.net.listener", "listener", "The handle for a network listener."),
            (NetworkWatchHandle, NetworkWatch, "host.net.watch", "network_watch", "The handle for one network watch stream."),
            (TimerHandle, Timer, "host.time.timer", "timer", "The handle for a scheduled timer."),
            (WatchHandle, Watch, "host.fs.watch", "watch", "The handle for a filesystem watcher."),
            (TraceHandle, Trace, "runtime.trace", "trace", "The handle for one trace session."),
            (TransferredHandle, Transferred, "runtime.resource.transferred", "transferred", "The handle for one transferred resource."),
        }
    };
}

pub(crate) use for_each_resource_handle_kind;

macro_rules! define_resource_kind {
    ($(($handle:ident, $kind:ident, $kind_id:literal, $label:literal, $doc:literal),)+) => {
        /// Resource classification for host handles.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum ResourceKind {
            $(
                #[doc = concat!("Resource kind for `", $label, "` handles.")]
                $kind,
            )+
        }

        impl ResourceKind {
            /// Return all built-in resource kinds.
            pub const fn all() -> &'static [Self] {
                &[
                    $(Self::$kind,)+
                ]
            }

            /// Return the stable kind identifier for topology and policy matching.
            pub const fn kind_id(self) -> &'static str {
                match self {
                    $(Self::$kind => $kind_id,)+
                }
            }

            /// Return the stable binding label for this kind.
            pub const fn label(self) -> &'static str {
                match self {
                    $(Self::$kind => $label,)+
                }
            }
        }
    };
}

for_each_resource_handle_kind!(define_resource_kind);
