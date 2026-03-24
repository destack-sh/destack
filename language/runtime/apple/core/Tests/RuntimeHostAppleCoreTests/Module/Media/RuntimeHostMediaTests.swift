import RuntimeHostAppleCore
import Testing

@Test
func testCreateMediaRequestsAndResult() {
    let listRequest = RuntimeHostMediaListRequest(
        limit: 10,
        kinds: [.image]
    )
    let listResult = RuntimeHostMediaListResult(assets: [
        RuntimeHostMediaAssetDescriptor(
            identifier: "asset-1",
            uri: "file:///tmp/example.png",
            filename: "example.png",
            mimeType: "image/png",
            kind: .image
        )
    ])
    let importRequest = RuntimeHostMediaImportPathRequest(
        path: "/tmp/example.png",
        kind: .image
    )
    let deleteRequest = RuntimeHostMediaDeleteRequest(identifiers: ["asset-1"])

    #expect(listRequest.kinds == [.image])
    #expect(listResult.assets.count == 1)
    #expect(listResult.assets[0].identifier == "asset-1")
    #expect(importRequest.path == "/tmp/example.png")
    #expect(deleteRequest.identifiers == ["asset-1"])
}
