import RuntimeHostAppleCore
import RuntimeHostMacOS
import Testing

@MainActor
@Test
func testApplicationCallbacksSendExpectedLifecycleStates() {
    let lifecycleEvents = MacOSRecordingLifecycleSink()
    let runtimeHost = createMacOSRuntimeHost(lifecycleEvents: lifecycleEvents)
    let adapter = ApplicationDelegate(runtimeHost: runtimeHost)

    adapter.applicationDidFinishLaunching()
    adapter.applicationDidBecomeActive()
    adapter.applicationWillResignActive()
    adapter.applicationWillTerminate()

    #expect(lifecycleEvents.events == [
        macOSApplicationLifecycleEvent(.initializing),
        macOSApplicationLifecycleEvent(.running),
        macOSApplicationLifecycleEvent(.paused),
        macOSApplicationLifecycleEvent(.destroyed),
    ])
}

@MainActor
@Test
func testWindowCallbacksSendExpectedLifecycleStates() {
    let lifecycleEvents = MacOSRecordingLifecycleSink()
    let runtimeHost = createMacOSRuntimeHost(
        lifecycleEvents: lifecycleEvents,
        surfaceKind: .metalView
    )
    let adapter = WindowDelegate(runtimeHost: runtimeHost)

    adapter.windowDidBecomeKey()
    adapter.windowDidResignKey()
    adapter.windowWillClose()

    #expect(lifecycleEvents.events == [
        windowLifecycleEvent(.running),
        windowLifecycleEvent(.paused),
        windowLifecycleEvent(.destroyed),
    ])
}
