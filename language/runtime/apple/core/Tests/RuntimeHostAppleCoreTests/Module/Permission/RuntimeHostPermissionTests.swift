import RuntimeHostAppleCore
import Testing

@Test
func testCreatePermissionRequestAndEvent() {
    let request = RuntimeHostPermissionRequest(
        requestID: HostRequestID(rawValue: 4),
        permission: "location"
    )
    let event = RuntimeHostPermissionEvent(
        requestID: HostRequestID(rawValue: 4),
        permission: "location",
        isGranted: true
    )

    #expect(request.requestID == HostRequestID(rawValue: 4))
    #expect(request.permission == "location")
    #expect(event.requestID == HostRequestID(rawValue: 4))
    #expect(event.permission == "location")
    #expect(event.isGranted)
}
