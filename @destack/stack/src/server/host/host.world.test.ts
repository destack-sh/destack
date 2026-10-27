import { expect, refusal, test } from "@destack/test";
import { present } from "@destack/schema";
import {
    type Capability,
    type ObjectType,
    PACKAGE_DECLARATION,
    PACKAGE_EXPORT,
    PackageHandle,
} from "@destack/object";
import { describePackage } from "@destack/object/inspect";
import objectPackage from "@destack/object/package";
import { implementPackage } from "@destack/object/server";
import type { InstalledServerContext } from "@destack/object/workload";
import { MemoryBuild } from "@destack/package/test";
import { Package } from "@destack/package";
import { testHost } from "@destack/test/host";
import { TestSpace } from "../../test/index.ts";

/** The one host the test package lists by name. */
const DECLARED_HOST = "api.example.com";

/** Install a package calling some hosts in a test space, answering the context it serves with. */
async function installCaller(connect: readonly string[]) {
    // ship a server declaring network access to the hosts, keeping the context its installation serves with
    const released = Package.parse({
        id: "package-01996ab0-0000-7000-8000-0000000000e2",
        name: "@example/caller",
        version: "2026.9.0",
    });
    const call: Capability = {
        reason: "Call the example API",
        sandbox: { network: { connect: [...connect] } },
    };
    const objects: Readonly<Record<string, ObjectType>> = {};
    const handle = PackageHandle.of(released, { objects, capabilities: { call } });
    const build = await MemoryBuild.of(
        Package.of(handle),
        [
            {
                kind: PACKAGE_DECLARATION,
                kindPackageId: objectPackage.id,
                name: PACKAGE_EXPORT,
                description: describePackage(handle),
            },
        ],
        testHost().crypto,
    );
    const contexts: InstalledServerContext[] = [];
    const fixture = await TestSpace.open({
        servers: [
            implementPackage(handle, (context) => {
                contexts.push(context);

                return { objects: [] };
            }),
        ],
        bundled: [build],
    });
    await fixture.install(await fixture.serve(), build);
    await fixture.serve();

    return { fixture, context: present(contexts.at(-1), "the installation's context") };
}

test("call only the hosts an in-store installation's capabilities declare, as an instance's sandbox does", async () => {
    const { context } = await installCaller([DECLARED_HOST]);

    // call an undeclared host and open a socket to another
    expect([
        await refusal(context.fetch(new Request("https://other.example.com/"))),
        await refusal(
            Promise.resolve().then(() =>
                context.environment.network.connect({ hostname: "smtp.example.com", port: 25 }),
            ),
        ),
    ]).toEqual([
        [
            "FORBIDDEN",
            "NETWORK_UNDECLARED",
            "installation caller declares no network access to other.example.com:443",
        ],
        [
            "FORBIDDEN",
            "NETWORK_UNDECLARED",
            "installation caller declares no network access to smtp.example.com:25",
        ],
    ]);
});

test("call a wildcard's public hosts alone, and follow redirects without credentials toward another origin, refusing a refused or body-replaying redirect", async () => {
    // serve a host redirecting to another origin, which records the headers it receives
    const { fixture, context } = await installCaller(["*", DECLARED_HOST]);
    const received: (string | null)[] = [];
    const network = fixture.simulation.network;
    network.server(DECLARED_HOST).http({
        hostname: DECLARED_HOST,
        port: 443,
        fetch: (request) =>
            new Response(null, {
                status: new URL(request.url).pathname === "/post" ? 307 : 302,
                headers: { location: "https://files.example.org/asset" },
            }),
    });
    network.server("files.example.org").http({
        hostname: "files.example.org",
        port: 443,
        fetch: (request) => {
            received.push(request.headers.get("authorization"));

            return new Response("asset");
        },
    });

    // fetch through the redirect with a credential, then refuse a refused and a body-replaying redirect and a private host
    const followed = await context.fetch(
        new Request(`https://${DECLARED_HOST}/asset`, { headers: { authorization: "Bearer key" } }),
    );
    expect([
        followed.status,
        await followed.text(),
        received,
        await refusal(
            context.fetch(new Request(`https://${DECLARED_HOST}/asset`, { redirect: "error" })),
        ),
        await refusal(
            context.fetch(
                new Request(`https://${DECLARED_HOST}/post`, { method: "POST", body: "{}" }),
            ),
        ),
        await refusal(context.fetch(new Request("http://localhost:8080/"))),
    ]).toEqual([
        200,
        "asset",
        [null],
        [
            "BAD_GATEWAY",
            "REDIRECT_REFUSED",
            `https://${DECLARED_HOST}/asset redirected, which the request refuses`,
        ],
        [
            "BAD_GATEWAY",
            "REDIRECT_REFUSED",
            `https://${DECLARED_HOST}/post redirected with 307, which would send its body again`,
        ],
        [
            "FORBIDDEN",
            "ADDRESS_PRIVATE",
            "localhost resolves to localhost, outside the public network",
        ],
    ]);
});
