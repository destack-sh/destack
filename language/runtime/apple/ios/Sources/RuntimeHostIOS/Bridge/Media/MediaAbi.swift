import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

/// One C callback for one media-list request.
typealias MediaListCallback =
  @convention(c) (
    UInt64,
    DestackRustMediaQuery,
    UnsafeMutablePointer<DestackRustMediaPage>?
  ) -> UInt32

/// One C callback for one media-read request.
typealias MediaReadCallback =
  @convention(c) (
    UInt64,
    DestackRustStringRef,
    UnsafeMutablePointer<DestackRustMediaAssetDescriptor>?
  ) -> UInt32

/// One C callback for one media-import request.
typealias MediaImportPathCallback =
  @convention(c) (
    UInt64,
    DestackRustStringRef,
    Int32,
    UnsafeMutablePointer<DestackRustStringRef>?
  ) -> UInt32

/// One C callback for one media-delete request.
typealias MediaDeleteCallback =
  @convention(c) (
    UInt64,
    DestackRustStringSlice,
    UnsafeMutablePointer<UInt32>?
  ) -> UInt32

/// One low-level media ABI for one iOS host bridge.
protocol MediaAbi {
}

extension ProcessRuntimeAbi {
}
