import RuntimeHostAppleCore

/// One no-op notification request handler for iOS tests.
@MainActor
final class IOSNoopNotificationRequestHandler: NotificationRequests {
    func postNotification(
        _ request: RuntimeHostNotificationRequest
    ) -> UInt32 {
        hostStatusNotSupported
    }

    func cancelNotification(
        identifier: String
    ) -> UInt32 {
        hostStatusNotSupported
    }

    func cancelAllNotifications() -> UInt32 {
        hostStatusNotSupported
    }
}

/// One recording notification request handler for iOS tests.
@MainActor
final class IOSRecordingNotificationRequestHandler: NotificationRequests {
    var postedRequests: [RuntimeHostNotificationRequest] = []
    var cancelledIdentifiers: [String] = []
    var cancelAllCalls: Int = 0
    var status: UInt32 = hostStatusOk

    func postNotification(
        _ request: RuntimeHostNotificationRequest
    ) -> UInt32 {
        postedRequests.append(request)

        return status
    }

    func cancelNotification(
        identifier: String
    ) -> UInt32 {
        cancelledIdentifiers.append(identifier)

        return status
    }

    func cancelAllNotifications() -> UInt32 {
        cancelAllCalls += 1

        return status
    }
}

/// One recording notification event sink for iOS tests.
@MainActor
final class IOSRecordingNotificationEventSink: NotificationEvents {
    var events: [RuntimeHostNotificationEvent] = []

    func sendNotificationEvent(_ event: RuntimeHostNotificationEvent) {
        events.append(event)
    }
}
