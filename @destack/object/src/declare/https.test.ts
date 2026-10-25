import { testSimulation } from "../test/index.ts";
import { ResourceId } from "@destack/resource";
import { expect, onTestFinished, test } from "@destack/test";
import { defineHttps } from "./https.ts";

test("call a lent HTTPS API below its egress address with the instance's credential, and refuse a binding without one", async () => {
    // answer each call with what it received
    const calls: [string, string | null][] = [];
    const world = testSimulation().environment("test");
    const server = world.server.http({
        hostname: "lent.test",
        fetch: (request) => {
            const url = new URL(request.url);
            calls.push([`${url.pathname}${url.search}`, request.headers.get("authorization")]);

            return Response.json({});
        },
    });
    onTestFinished(() => server.stop(true));

    // connect through the lent connector as the host binds it
    const github = defineHttps("github", { url: "https://api.github.com" });
    const binding = {
        resource: ResourceId.parse("https-01996ab0-0000-7000-8000-000000000001"),
        kind: "https",
        provider: "connection",
        reference: `${server.url.origin}/.destack/egress/https-01996ab0-0000-7000-8000-000000000001/`,
    };
    const client = await github.connectors.connection.connect(
        { ...binding, credential: "instance-secret" },
        github,
        world,
    );
    await client.fetch("/repos/acme/app?page=2");

    // refuse a binding without the instance's credential
    await expect(github.connectors.connection.connect(binding, github, world)).rejects.toThrow(
        new TypeError("https binding github has no credential"),
    );
    expect({
        calls,
        spec: github.spec,
    }).toEqual({
        calls: [
            [
                "/.destack/egress/https-01996ab0-0000-7000-8000-000000000001/repos/acme/app?page=2",
                "Bearer instance-secret",
            ],
        ],
        spec: { url: "https://api.github.com" },
    });
});
