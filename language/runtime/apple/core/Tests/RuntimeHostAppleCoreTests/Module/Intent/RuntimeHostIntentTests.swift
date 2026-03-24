import RuntimeHostAppleCore
import Testing

@Test
func testCreateIntentEvent() {
    let event = RuntimeHostIntentEvent(
        source: "com.example.host",
        payload: .openURL(url: "https://destack.dev")
    )

    #expect(event.source == "com.example.host")
    #expect(event.payload == .openURL(url: "https://destack.dev"))
}
