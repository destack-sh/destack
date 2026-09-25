use crate::tests::{DirRows, TestSession};

#[test]
fn test_apply_a_binding_decorator_to_an_interface_method() {
    let session = TestSession::single(
        r#"
struct ResourceId {}

struct IoControlRequest {}

interface IoControlBinding {
    @binding("tspp.io.control", {
        provider: "host",
        effect: "external",
        requires: ["host.fs.metadata"],
        families: ["windows", "unix"],
    })
    executeIoControl(resource: ResourceId, request: IoControlRequest): IoControlRequest;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct ResourceId {}

struct IoControlRequest {}

interface IoControlBinding {
    @binding("tspp.io.control", {
        provider: "host",
        effect: "external",
        requires: ["host.fs.metadata"],
        families: ["windows", "unix"],
    })
    executeIoControl(resource: ResourceId, request: IoControlRequest): IoControlRequest;
}

=== dir ===
struct ResourceId {}
/// @type.symbol symbol=ResourceId source="struct ResourceId {}" type=ResourceId
/// @definition.struct symbol=ResourceId source="struct ResourceId {}"

struct IoControlRequest {}
/// @type.symbol symbol=IoControlRequest source="struct IoControlRequest {}" type=IoControlRequest
/// @definition.struct symbol=IoControlRequest source="struct IoControlRequest {}"

interface IoControlBinding {
/// @generic.template symbol=IoControlBinding parameters=(this: IoControlBinding)
/// @type.symbol symbol=IoControlBinding type=IoControlBinding
/// @definition.interface symbol=IoControlBinding template=(this: IoControlBinding)
/// @definition.where symbol=IoControlBinding relation=satisfies left=this right=IoControlBinding
/// @definition.method symbol=IoControlBinding.executeIoControl slot=executeIoControl type=(ResourceId, IoControlRequest) => IoControlRequest

    @binding("tspp.io.control", {
    /// @resolution.name source=binding target=binding

        provider: "host",
        effect: "external",
        requires: ["host.fs.metadata"],
        families: ["windows", "unix"],
    })
    executeIoControl(resource: ResourceId, request: IoControlRequest): IoControlRequest;
    /// @type.symbol symbol=IoControlBinding.executeIoControl type=(ResourceId, IoControlRequest) => IoControlRequest
    /// @type.symbol symbol=IoControlBinding.executeIoControl.resource source="resource: ResourceId" type=ResourceId
    /// @resolution.name source=ResourceId target=ResourceId
    /// @type.symbol symbol=IoControlBinding.executeIoControl.request source="request: IoControlRequest" type=IoControlRequest
    /// @resolution.name source=IoControlRequest target=IoControlRequest
    /// @resolution.name source=IoControlRequest target=IoControlRequest

}
"#,
        r#"
"#,
    );
}
