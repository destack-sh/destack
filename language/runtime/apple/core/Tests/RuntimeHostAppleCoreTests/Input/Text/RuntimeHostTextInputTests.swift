import RuntimeHostAppleCore
import Testing

@Test
func testCreateTextInputRequestsAndEvent() {
    let configuration = RuntimeHostTextInputConfiguration(
        identifier: "editor",
        isMultiline: true,
        isSecure: false
    )
    let openRequest = RuntimeHostTextInputOpenRequest(configuration: configuration)
    let closeRequest = RuntimeHostTextInputCloseRequest(identifier: "editor")
    let event = RuntimeHostTextInputEvent(
        identifier: "editor",
        text: "hello",
        selection: RuntimeHostTextSelectionRange(start: 1, end: 4),
        composing: RuntimeHostTextSelectionRange(start: 1, end: 5)
    )

    #expect(openRequest.configuration.identifier == "editor")
    #expect(openRequest.configuration.isMultiline)
    #expect(closeRequest.identifier == "editor")
    #expect(event.text == "hello")
    #expect(event.selection == RuntimeHostTextSelectionRange(start: 1, end: 4))
    #expect(event.composing == RuntimeHostTextSelectionRange(start: 1, end: 5))
}
