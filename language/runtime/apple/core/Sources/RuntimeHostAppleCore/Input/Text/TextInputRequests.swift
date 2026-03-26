import Foundation

/// The text-input request surface attached to one Apple runtime host.
@MainActor
public protocol TextInputRequests {
  /// Open one Apple text-input session.
  func openTextInput(
    _ request: RuntimeHostTextInputOpenRequest
  ) -> UInt32

  /// Close one Apple text-input session.
  func closeTextInput(
    _ request: RuntimeHostTextInputCloseRequest
  ) -> UInt32

  /// Update one Apple text-input geometry hint.
  func setTextInputGeometry(
    _ request: RuntimeHostTextInputGeometryRequest
  ) -> UInt32

  /// Update one Apple text-input state.
  func setTextInputState(
    _ request: RuntimeHostTextInputStateRequest
  ) -> UInt32
}

/// The explicit unsupported text-input request surface for one Apple runtime host.
@MainActor
public final class UnsupportedTextInputRequests: TextInputRequests {
  /// Create one unsupported text-input request surface.
  public init() {}

  public func openTextInput(
    _ request: RuntimeHostTextInputOpenRequest
  ) -> UInt32 {
    let _ = request

    return hostStatusNotSupported
  }

  public func closeTextInput(
    _ request: RuntimeHostTextInputCloseRequest
  ) -> UInt32 {
    let _ = request

    return hostStatusNotSupported
  }

  public func setTextInputGeometry(
    _ request: RuntimeHostTextInputGeometryRequest
  ) -> UInt32 {
    let _ = request

    return hostStatusNotSupported
  }

  public func setTextInputState(
    _ request: RuntimeHostTextInputStateRequest
  ) -> UInt32 {
    let _ = request

    return hostStatusNotSupported
  }
}
