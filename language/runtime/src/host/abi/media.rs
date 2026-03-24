use std::ptr;

use crate::diagnostic::RuntimeResult;
use crate::host::core::callback::decode_callback_host_required_string_ref;
use crate::platform::os::MediaAssetKind;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaPageValue, MediaQueryValue,
};
use crate::runtime::{NativeSlice, NativeStringRef};

/// A media asset kind.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostMediaAssetKind {
    /// An image asset.
    Image = 1,
    /// A video asset.
    Video = 2,
    /// An audio asset.
    Audio = 3,
    /// A non-standard asset.
    Other = 4,
}

impl HostMediaAssetKind {
    /// Decode one raw host media kind.
    pub(crate) const fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            1 => Some(Self::Image),
            2 => Some(Self::Video),
            3 => Some(Self::Audio),
            4 => Some(Self::Other),
            _ => None,
        }
    }

    /// Convert one raw host media kind into one runtime media kind.
    pub(crate) const fn into_media_kind(self) -> MediaAssetKind {
        match self {
            Self::Image => MediaAssetKind::Image,
            Self::Video => MediaAssetKind::Video,
            Self::Audio => MediaAssetKind::Audio,
            Self::Other => MediaAssetKind::Other,
        }
    }
}

impl From<MediaAssetKind> for HostMediaAssetKind {
    fn from(kind: MediaAssetKind) -> Self {
        match kind {
            MediaAssetKind::Image => Self::Image,
            MediaAssetKind::Video => Self::Video,
            MediaAssetKind::Audio => Self::Audio,
            MediaAssetKind::Other => Self::Other,
        }
    }
}

impl From<HostMediaAssetKind> for MediaAssetKind {
    fn from(kind: HostMediaAssetKind) -> Self {
        kind.into_media_kind()
    }
}

/// One owned media-query payload.
#[derive(Debug)]
pub(crate) struct HostMediaQueryPayload {
    /// The owned cursor backing storage.
    cursor: String,
    /// The owned kind backing storage.
    kinds: Vec<HostMediaAssetKind>,
    /// The borrowed ABI query view.
    abi: HostMediaQuery,
}

impl From<&MediaQueryValue> for HostMediaQueryPayload {
    fn from(query: &MediaQueryValue) -> Self {
        let cursor = query.cursor.clone().unwrap_or_default();
        let kinds = query
            .kinds
            .iter()
            .copied()
            .map(HostMediaAssetKind::from)
            .collect::<Vec<_>>();

        let abi = HostMediaQuery {
            has_cursor: query.cursor.is_some(),
            cursor: NativeStringRef::from(&cursor),
            has_limit: query.limit.is_some(),
            limit: query.limit.unwrap_or_default(),
            kinds: media_kind_slice(&kinds),
            include_hidden: query.include_hidden,
        };

        Self { cursor, kinds, abi }
    }
}

impl HostMediaQueryPayload {
    /// Return the ABI query view.
    pub(crate) fn abi(&self) -> HostMediaQuery {
        let _ = &self.cursor;
        let _ = &self.kinds;

        self.abi
    }
}

impl HostMediaAssetDescriptor {
    /// Decode one host media descriptor.
    pub(crate) fn decode(
        self,
        operation: &'static str,
    ) -> RuntimeResult<MediaAssetDescriptorValue> {
        Ok(MediaAssetDescriptorValue {
            id: decode_callback_host_required_string_ref(
                operation,
                "HostMediaAssetDescriptor.id",
                self.id,
            )?,
            uri: decode_callback_host_required_string_ref(
                operation,
                "HostMediaAssetDescriptor.uri",
                self.uri,
            )?,
            filename: decode_callback_host_required_string_ref(
                operation,
                "HostMediaAssetDescriptor.filename",
                self.filename,
            )?,
            mime_type: decode_callback_host_required_string_ref(
                operation,
                "HostMediaAssetDescriptor.mime_type",
                self.mime_type,
            )?,
            kind: self.kind.into(),
            width: self.width,
            height: self.height,
            duration_ms: self.duration_ms,
            size_bytes: self.size_bytes,
            created_unix_ns: self.created_unix_ns,
            modified_unix_ns: self.modified_unix_ns,
        })
    }
}

impl HostMediaPage {
    /// Decode one host media page.
    pub(crate) fn decode(self, operation: &'static str) -> RuntimeResult<MediaPageValue> {
        // assets
        let assets = unsafe { self.assets.as_slice() }?;
        let assets = assets
            .iter()
            .copied()
            .map(|asset| asset.decode(operation))
            .collect::<RuntimeResult<Vec<_>>>()?;

        // next cursor
        let next_cursor = if self.has_next_cursor {
            decode_callback_host_required_string_ref(
                operation,
                "HostMediaPage.next_cursor",
                self.next_cursor,
            )?
        } else {
            String::new()
        };

        Ok(MediaPageValue {
            assets,
            next_cursor,
            has_more: self.has_more,
        })
    }
}

/// A slice of media asset kinds.
pub(crate) type HostMediaAssetKindSlice = NativeSlice<HostMediaAssetKind>;

/// A media library query.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct HostMediaQuery {
    /// Whether the query includes one cursor.
    pub(crate) has_cursor: bool,
    /// The opaque cursor from one prior media list call.
    pub(crate) cursor: NativeStringRef,
    /// Whether the query includes one limit.
    pub(crate) has_limit: bool,
    /// The maximum returned assets for this page.
    pub(crate) limit: u32,
    /// The requested asset kinds.
    pub(crate) kinds: HostMediaAssetKindSlice,
    /// Whether hidden assets should be included.
    pub(crate) include_hidden: bool,
}

/// A media asset descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct HostMediaAssetDescriptor {
    /// The stable asset identifier.
    pub(crate) id: NativeStringRef,
    /// The host URI for this asset.
    pub(crate) uri: NativeStringRef,
    /// The asset filename.
    pub(crate) filename: NativeStringRef,
    /// The asset MIME type when available.
    pub(crate) mime_type: NativeStringRef,
    /// The asset class.
    pub(crate) kind: HostMediaAssetKind,
    /// The asset width in pixels when available.
    pub(crate) width: u32,
    /// The asset height in pixels when available.
    pub(crate) height: u32,
    /// The asset duration in milliseconds for time-based assets.
    pub(crate) duration_ms: u64,
    /// The asset size in bytes when available.
    pub(crate) size_bytes: u64,
    /// The asset creation timestamp in UTC nanoseconds when available.
    pub(crate) created_unix_ns: u64,
    /// The asset modification timestamp in UTC nanoseconds when available.
    pub(crate) modified_unix_ns: u64,
}

/// A slice of media asset descriptors.
pub(crate) type HostMediaAssetDescriptorSlice = NativeSlice<HostMediaAssetDescriptor>;

/// A page of media assets returned by the host.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct HostMediaPage {
    /// The returned assets for this page.
    pub(crate) assets: HostMediaAssetDescriptorSlice,
    /// Whether the page includes one next cursor.
    pub(crate) has_next_cursor: bool,
    /// The opaque next-page cursor when available.
    pub(crate) next_cursor: NativeStringRef,
    /// Whether more assets are available.
    pub(crate) has_more: bool,
}

/// Build one media-kind slice.
fn media_kind_slice(values: &[HostMediaAssetKind]) -> HostMediaAssetKindSlice {
    let data = if values.is_empty() {
        ptr::null_mut()
    } else {
        values.as_ptr() as *mut HostMediaAssetKind
    };

    HostMediaAssetKindSlice {
        data,
        len: values.len() as u32,
    }
}
