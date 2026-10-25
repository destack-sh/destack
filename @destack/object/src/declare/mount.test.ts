import { expect, test } from "@destack/test";
import { DeviceMount, PackageMount } from "./mount.ts";

test("mount each package under a separate path and route requests below it", async () => {
    // build a mount and route requests inside and outside it
    const path = PackageMount.path("package-01");
    const url = PackageMount.url("http://127.0.0.1:4100/", "package-01");
    const sent = new Request(`http://127.0.0.1${path}/notes/list?limit=2`, {
        method: "POST",
        body: "{}",
    });
    const routed = PackageMount.of(sent);
    const root = PackageMount.of(new Request(`http://127.0.0.1${path}`));
    expect({
        path,
        url,
        routed: [
            routed?.packageId,
            routed?.request.url,
            routed?.request.method,
            await routed?.request.text(),
        ],
        root: [root?.packageId, root?.request.url],
        outside: PackageMount.of(new Request("http://127.0.0.1/view/notes")),
    }).toEqual({
        path: "/package-01",
        url: "http://127.0.0.1:4100/package-01",
        routed: ["package-01", "http://127.0.0.1/notes/list?limit=2", "POST", "{}"],
        root: ["package-01", "http://127.0.0.1/"],
        outside: undefined,
    });
});

test("mount a native device's local objects under their path and split paths below it", () => {
    const path = DeviceMount.path("device-01");

    expect({
        path,
        below: DeviceMount.parse(`${path}/store/call`),
        root: DeviceMount.parse(path),
        outside: DeviceMount.parse("/spaces/space-01/package-01"),
    }).toEqual({
        path: "/devices/device-01",
        below: { deviceId: "device-01", path: "/store/call" },
        root: { deviceId: "device-01", path: "/" },
        outside: undefined,
    });
});
