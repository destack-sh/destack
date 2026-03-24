import Foundation
import RuntimeHostAppleCore

/// The application embedder for one iOS runtime host.
@MainActor
public final class ApplicationEmbedder {
    /// The iOS runtime host attached to this application.
    public let runtimeHost: RuntimeHost

    /// The application delegate adapter installed for this embedder.
    public let applicationDelegate: ApplicationDelegate

    /// Create one iOS application embedder.
    public init(
        runtimeHost: RuntimeHost
    ) {
        self.runtimeHost = runtimeHost
        self.applicationDelegate = ApplicationDelegate(runtimeHost: runtimeHost)
    }
}
