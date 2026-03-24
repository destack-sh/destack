import Foundation

/// The lifecycle ingress surface attached to one Apple runtime host.
@MainActor
public protocol LifecycleEvents {
    /// Send one normalized lifecycle event into the attached runtime session.
    func sendLifecycleEvent(_ event: RuntimeHostLifecycleEvent)
}
