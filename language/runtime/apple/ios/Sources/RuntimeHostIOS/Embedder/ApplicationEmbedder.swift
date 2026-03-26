import Foundation
import RuntimeHostAppleCore

/// The application embedder for one iOS runtime host.
@MainActor
public final class ApplicationEmbedder {
  /// The iOS runtime host attached to this application.
  public let runtimeHost: RuntimeHost

  /// The application delegate adapter installed for this embedder.
  public let applicationDelegate: ApplicationDelegate
  /// The background request surface attached to this application.
  public let backgroundRequests: any BackgroundRequests

  /// Create one iOS application embedder.
  public init(
    runtimeHost: RuntimeHost
  ) {
    let backgroundRequests = BackgroundTaskSchedulerRequests(
      events: runtimeHost.backgroundEvents
    )

    self.runtimeHost = runtimeHost
    self.applicationDelegate = ApplicationDelegate(runtimeHost: runtimeHost)
    self.backgroundRequests = backgroundRequests
    runtimeHost.backgroundRequests = backgroundRequests
  }
}
