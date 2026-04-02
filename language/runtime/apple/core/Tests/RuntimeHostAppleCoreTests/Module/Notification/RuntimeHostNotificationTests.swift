import RuntimeHostAppleCore
import Testing

@Test
func testCreateNotificationRequestAndEvent() {
  let request = RuntimeHostNotificationRequest(
    title: "Hello",
    body: "Destack is ready",
    tag: "welcome"
  )
  let event = RuntimeHostNotificationEvent(
    kind: .interacted,
    metadata: RuntimeHostNotificationEventMetadata(
      id: "welcome",
      request: request
    ),
    payload: RuntimeHostNotificationInteractedPayload(
      actionId: "open"
    )
  )

  #expect(request.identifier == "welcome")
  #expect(request.title == "Hello")
  #expect(event.identifier == "welcome")
  #expect(event.kind == .interacted)
  #expect(event.actionIdentifier == "open")
}
