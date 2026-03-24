import RuntimeHostAppleCore
import Testing

@testable import RuntimeHostIOS

@MainActor
@Test
func testApplicationLifecycleEventMapsToApplicationSource() {
  #expect(
    applicationLifecycleEvent(.running)
      == RuntimeHostLifecycleEvent(
        sourceKind: .application,
        state: .running
      ))
}

@MainActor
@Test
func testSceneLifecycleEventMapsToSceneSource() {
  #expect(
    sceneLifecycleEvent(.running)
      == RuntimeHostLifecycleEvent(
        sourceKind: .scene,
        state: .running
      ))
}
