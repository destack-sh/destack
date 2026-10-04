import { expect, test } from "@destack/test";
import { originalRequest } from "../request/request.ts";
import { ServiceMount } from "./mount.ts";

test("mount each package's service under its own path and route requests below it, keeping the request as sent", async () => {
    // name a mount and route requests inside and outside it
    const path = ServiceMount.path("package-01");
    const url = ServiceMount.url("http://127.0.0.1:4100/", "package-01");
    const sent = new Request(`http://127.0.0.1${path}/notes/list?limit=2`, {
        method: "POST",
        body: "{}",
    });
    const routed = ServiceMount.route(sent);
    const root = ServiceMount.route(new Request(`http://127.0.0.1${path}`));
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
        isOriginalKept: routed !== undefined && originalRequest(routed.request) === sent,
        outside: ServiceMount.route(new Request("http://127.0.0.1/view/notes")),
    }).toEqual({
        path: "/service/package-01",
        url: "http://127.0.0.1:4100/service/package-01",
        routed: ["package-01", "http://127.0.0.1/notes/list?limit=2", "POST", "{}"],
        root: ["package-01", "http://127.0.0.1/"],
        isOriginalKept: true,
        outside: undefined,
    });
});
