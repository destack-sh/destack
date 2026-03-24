import RuntimeHostAppleCore
import RuntimeHostIOS
import Testing

@MainActor
@Test
func testApplicationCallbacksSendExpectedLifecycleStates() {
    let lifecycleEvents = IOSRecordingLifecycleSink()
    let runtimeHost = createIOSRuntimeHost(lifecycleEvents: lifecycleEvents)
    let adapter = ApplicationDelegate(runtimeHost: runtimeHost)

    #expect(adapter.applicationDidFinishLaunching())
    adapter.applicationDidBecomeActive()
    adapter.applicationWillResignActive()
    adapter.applicationDidEnterBackground()
    adapter.applicationWillEnterForeground()
    adapter.applicationWillTerminate()

    #expect(lifecycleEvents.events == [
        applicationLifecycleEvent(.initializing),
        applicationLifecycleEvent(.running),
        applicationLifecycleEvent(.paused),
        applicationLifecycleEvent(.paused),
        applicationLifecycleEvent(.running),
        applicationLifecycleEvent(.destroyed),
    ])
}

@MainActor
@Test
func testSceneCallbacksSendExpectedLifecycleStates() {
    let lifecycleEvents = IOSRecordingLifecycleSink()
    let runtimeHost = createIOSRuntimeHost(
        lifecycleEvents: lifecycleEvents,
        surfaceKind: .metalView
    )
    let adapter = SceneDelegate(runtimeHost: runtimeHost)

    adapter.sceneWillConnect()
    adapter.sceneDidBecomeActive()
    adapter.sceneWillResignActive()
    adapter.sceneDidEnterBackground()
    adapter.sceneWillEnterForeground()
    adapter.sceneDidDisconnect()

    #expect(lifecycleEvents.events == [
        sceneLifecycleEvent(.initializing),
        sceneLifecycleEvent(.running),
        sceneLifecycleEvent(.paused),
        sceneLifecycleEvent(.paused),
        sceneLifecycleEvent(.running),
        sceneLifecycleEvent(.destroyed),
    ])
}
