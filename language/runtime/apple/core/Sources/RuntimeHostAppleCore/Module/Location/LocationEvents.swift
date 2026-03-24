import Foundation

/// The location ingress surface attached to one Apple runtime host.
public protocol LocationEvents {
  /// Send one normalized location sample into the attached runtime session.
  func sendLocationSample(
    watchID: String,
    sample: RuntimeHostLocationSample
  )
}
