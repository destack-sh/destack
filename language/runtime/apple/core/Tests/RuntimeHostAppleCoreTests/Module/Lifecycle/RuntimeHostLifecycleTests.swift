import RuntimeHostAppleCore
import Testing

@Test
func testCreateLifecycleEvent() {
    let event = lifecycleEvent(.application, .running)

    #expect(event.sourceKind == .application)
    #expect(event.state == .running)
}
