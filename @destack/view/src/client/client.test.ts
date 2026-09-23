import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { defineProcedure } from "@destack/service";
import { defineServiceConnection } from "@destack/service/declare";
import { openClient, type ViewBrowser } from "./client.ts";

test("bootstrap declared clients within the browser origin and reject foreign endpoints", async () => {
    // retain tab credentials while removing them from visible navigation
    const packageId = PackageId.parse("package-019f7480-0000-7000-8000-000000000001");
    const router = {
        read: defineProcedure({ authentication: "public", permission: null, audit: false })
            .route({ method: "GET", path: "/value" })
            .output(schema.string()),
    };
    const connection = defineServiceConnection(
        { packageId, name: "notes", service: { packageId, name: "notes" } },
        router,
    );
    const storage = new Map<string, string>();
    const history: unknown[][] = [];
    const requests: { url: string; authorization: string | null; redirect: RequestRedirect }[] = [];
    let endpoint = "/service/notes";
    const browser: ViewBrowser = {
        location: {
            hash: "#window-secret",
            origin: "https://app.test",
            pathname: "/notes",
            search: "?sort=title",
        },
        sessionStorage: {
            getItem: (key) => storage.get(key) ?? null,
            setItem: (key, value) => {
                storage.set(key, value);
            },
        },
        history: {
            replaceState: (...arguments_) => {
                history.push(arguments_);
            },
        },
        fetch: async (input, options) => {
            const request = new Request(input, options);
            requests.push({
                url: request.url,
                authorization: request.headers.get("authorization"),
                redirect: request.redirect,
            });
            if (new URL(request.url).pathname === "/view") {
                return Response.json({
                    packageId,
                    services: [{ declaration: { packageId, name: "notes" }, url: endpoint }],
                });
            }

            return Response.json("saved note");
        },
    };

    // use the ordinary service handle after browser bootstrap
    const context = await openClient(browser);
    context.bind(connection);
    expect(await connection.get(context.resources).read()).toBe("saved note");
    expect(history).toEqual([[null, "", "/notes?sort=title"]]);
    expect(requests).toEqual([
        { url: "https://app.test/view", authorization: "Bearer window-secret", redirect: "error" },
        {
            url: "https://app.test/service/notes/value",
            authorization: "Bearer window-secret",
            redirect: "error",
        },
    ]);

    // restore the tab credential without allowing host configuration to redirect it
    browser.location.hash = "";
    for (const url of [
        "https://foreign.test/service",
        "//foreign.test/service",
        "https://user:password@app.test/service",
        "/service#credential",
    ]) {
        endpoint = url;
        await expect(openClient(browser)).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "view service must use the application origin",
        });
    }
    expect(requests.slice(2)).toEqual(
        Array.from({ length: 4 }, () => ({
            url: "https://app.test/view",
            authorization: "Bearer window-secret",
            redirect: "error",
        })),
    );
});
