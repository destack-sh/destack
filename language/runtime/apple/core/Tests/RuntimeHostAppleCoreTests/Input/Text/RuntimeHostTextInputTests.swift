import RuntimeHostAppleCore
import Testing

@Test
func testCreateTextInputRequestsAndEvent() {
  let configuration = RuntimeHostTextInputConfiguration(
    sessionID: 7,
    inputType: .text,
    isMultiline: true,
    isSecure: false
  )
  let state = RuntimeHostTextInputState(
    text: "hello",
    selection: RuntimeHostTextInputRange(startOffset: 1, endOffset: 4),
    composing: RuntimeHostTextInputRange(startOffset: 1, endOffset: 5)
  )
  let geometry = RuntimeHostTextInputGeometry(
    localToTargetTransform: RuntimeHostTextInputTransform2D(
      xx: 1,
      xy: 0,
      yx: 0,
      yy: 1,
      tx: 0,
      ty: 0
    ),
    editorRectangle: RuntimeHostTextInputRectangle(
      x: 10,
      y: 20,
      width: 300,
      height: 120
    ),
    caretRectangle: RuntimeHostTextInputRectangle(
      x: 16,
      y: 24,
      width: 2,
      height: 18
    )
  )
  let openRequest = RuntimeHostTextInputOpenRequest(
    configuration: configuration,
    state: state
  )
  let closeRequest = RuntimeHostTextInputCloseRequest(sessionID: 7)
  let geometryRequest = RuntimeHostTextInputGeometryRequest(
    sessionID: 7,
    geometry: geometry
  )
  let stateRequest = RuntimeHostTextInputStateRequest(sessionID: 7, state: state)
  let event = RuntimeHostTextInputEvent(
    sessionID: 7,
    state: state
  )

  #expect(openRequest.configuration.sessionID == 7)
  #expect(openRequest.configuration.inputType == .text)
  #expect(openRequest.configuration.isMultiline)
  #expect(openRequest.state.text == "hello")
  #expect(closeRequest.sessionID == 7)
  #expect(geometryRequest.sessionID == 7)
  #expect(geometryRequest.geometry == geometry)
  #expect(stateRequest.sessionID == 7)
  #expect(stateRequest.state == state)
  #expect(event.sessionID == 7)
  #expect(event.state.text == "hello")
  #expect(event.state.selection == RuntimeHostTextInputRange(startOffset: 1, endOffset: 4))
  #expect(event.state.composing == RuntimeHostTextInputRange(startOffset: 1, endOffset: 5))
}
