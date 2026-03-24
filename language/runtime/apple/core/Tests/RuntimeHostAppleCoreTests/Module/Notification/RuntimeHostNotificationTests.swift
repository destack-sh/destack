import RuntimeHostAppleCore
import Testing

@Test
func testCreateNotificationRequestAndEvent() {
    let request = RuntimeHostNotificationRequest(
        identifier: "welcome",
        title: "Hello",
        body: "Destack is ready"
    )
    let event = RuntimeHostNotificationEvent(
        identifier: "welcome",
        request: request,
        kind: .activated,
        actionIdentifier: "open"
    )

    #expect(request.identifier == "welcome")
    #expect(request.title == "Hello")
    #expect(event.identifier == "welcome")
    #expect(event.kind == .activated)
    #expect(event.actionIdentifier == "open")
}
