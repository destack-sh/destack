import Foundation

/// The permission ingress surface attached to one Apple runtime host.
@MainActor
public protocol PermissionEvents {
    /// Send one normalized permission event into the attached runtime session.
    func sendPermissionEvent(_ event: RuntimeHostPermissionEvent)
}
