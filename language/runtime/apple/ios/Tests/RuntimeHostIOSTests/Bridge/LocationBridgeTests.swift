import RuntimeHostAppleCore
import Testing

@testable import RuntimeHostIOS

@MainActor
@Test
func testLocationRequestsAndIngressRouteIntoRuntimeHost() throws {
  let bindings = RuntimeAbiSpy()
  let sessionHandle = HostSessionHandle(rawValue: 7)
  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    bindings: bindings
  )
  let locationRequests = LocationRequestRecorder()
  locationRequests.servicesEnabledResponse = RuntimeHostLocationServicesResponse(
    status: hostStatusOk,
    isEnabled: true
  )
  locationRequests.lastKnownResponse = RuntimeHostLocationLastKnownResponse(
    status: hostStatusOk,
    sample: makeIOSLocationSample()
  )
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: PermissionRequestRecorder(),
    documentRequests: DocumentRequestRecorder(),
    intentRequests: IntentRequestRecorder(),
    locationRequests: locationRequests
  )

  try bridge.attach(runtimeHost: runtimeHost)

  let servicesEnabled = bridge.locationBridge.locationServicesEnabled(
    runtimeHost: runtimeHost
  )
  let lastKnown = bridge.locationBridge.locationLastKnown(
    runtimeHost: runtimeHost
  )
  let openStatus = bridge.locationBridge.locationWatchOpen(
    runtimeHost: runtimeHost,
    watchID: "watch-1",
    options: RuntimeHostLocationWatchOptions(
      accuracy: .high,
      minimumIntervalNs: 50_000_000,
      minimumDistanceMeters: 2.5,
      includeHeading: true
    )
  )
  bridge.sendLocationSample(
    watchID: "watch-1",
    sample: makeIOSLocationSample()
  )
  let closeStatus = bridge.locationBridge.locationWatchClose(
    runtimeHost: runtimeHost,
    watchID: "watch-1"
  )

  #expect(
    servicesEnabled == RuntimeHostLocationServicesResponse(status: hostStatusOk, isEnabled: true))
  #expect(
    lastKnown
      == RuntimeHostLocationLastKnownResponse(status: hostStatusOk, sample: makeIOSLocationSample())
  )
  #expect(locationRequests.watchOpenCalls.count == 1)
  #expect(locationRequests.watchOpenCalls[0].0 == "watch-1")
  #expect(
    locationRequests.watchOpenCalls[0].1
      == RuntimeHostLocationWatchOptions(
        accuracy: .high,
        minimumIntervalNs: 50_000_000,
        minimumDistanceMeters: 2.5,
        includeHeading: true
      ))
  #expect(locationRequests.watchCloseCalls == ["watch-1"])
  #expect(openStatus == hostStatusOk)
  #expect(closeStatus == hostStatusOk)
  #expect(bindings.locationSamples.count == 1)
  #expect(bindings.locationSamples[0].0 == sessionHandle)
  #expect(bindings.locationSamples[0].1 == "watch-1")
  #expect(bindings.locationSamples[0].2 == makeIOSLocationSample())
}
