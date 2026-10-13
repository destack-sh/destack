import { expect, test } from "@destack/test";
import { ClientMount, ServiceMount } from "./mount.ts";

test("mount each package's service under its own path and route requests below it", async () => {
    // build a mount and route requests inside and outside it
    const path = ServiceMount.path("package-01");
    const url = ServiceMount.url("http://127.0.0.1:4100/", "package-01");
    const sent = new Request(`http://127.0.0.1${path}/notes/list?limit=2`, {
        method: "POST",
        body: "{}",
    });
    const routed = ServiceMount.of(sent);
    const root = ServiceMount.of(new Request(`http://127.0.0.1${path}`));
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
        outside: ServiceMount.of(new Request("http://127.0.0.1/view/notes")),
    }).toEqual({
        path: "/service/package-01",
        url: "http://127.0.0.1:4100/service/package-01",
        routed: ["package-01", "http://127.0.0.1/notes/list?limit=2", "POST", "{}"],
        root: ["package-01", "http://127.0.0.1/"],
        outside: undefined,
    });
});

test("mount a native client's local objects under its own path and split paths below it", () => {
    const path = ClientMount.path("client-01");

    expect({
        path,
        below: ClientMount.parse(`${path}/replica/call`),
        root: ClientMount.parse(path),
        outside: ClientMount.parse("/spaces/space-01/service/package-01"),
    }).toEqual({
        path: "/clients/client-01",
        below: { clientId: "client-01", path: "/replica/call" },
        root: { clientId: "client-01", path: "/" },
        outside: undefined,
    });
});
