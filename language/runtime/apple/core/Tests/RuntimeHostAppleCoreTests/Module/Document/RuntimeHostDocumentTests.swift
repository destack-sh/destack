import RuntimeHostAppleCore
import Testing

@Test
func testCreateDocumentRequestAndResult() {
  let request = RuntimeHostDocumentRequest(
    requestID: HostRequestID(rawValue: 5),
    allowsMultipleSelection: true,
    contentTypes: ["image/png", "image/jpeg"]
  )
  let result = RuntimeHostDocumentResult(
    requestID: HostRequestID(rawValue: 5),
    documents: [
      RuntimeHostDocumentDescriptor(
        uri: "file:///tmp/example.png",
        displayName: "example.png",
        contentType: "image/png",
        localPath: "/tmp/example.png"
      )
    ])

  #expect(request.requestID == HostRequestID(rawValue: 5))
  #expect(request.allowsMultipleSelection)
  #expect(request.contentTypes == ["image/png", "image/jpeg"])
  #expect(result.requestID == HostRequestID(rawValue: 5))
  #expect(result.documents.count == 1)
  #expect(result.documents[0].displayName == "example.png")
}
