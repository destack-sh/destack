import { expect, test } from "@destack/test";
import { Sandbox } from "./sandbox.ts";

test("merge sandboxes into every ability of each, uniting host and command lists and taking writing over reading", () => {
    // join an indexer's base sandbox with what two of its capabilities unlock
    const base = Sandbox.parse({
        process: {},
        network: { connect: ["api.github.com"] },
        run: { commands: ["git"] },
        files: { access: "read" },
    });
    const importing = Sandbox.parse({
        network: { connect: ["gmail.googleapis.com", "api.github.com"] },
    });
    const syncing = Sandbox.parse({ files: { access: "write" }, camera: {} });

    expect([Sandbox.merge(), Sandbox.merge(base), Sandbox.merge(base, importing, syncing)]).toEqual(
        [
            {},
            base,
            {
                process: {},
                network: { connect: ["api.github.com", "gmail.googleapis.com"] },
                run: { commands: ["git"] },
                files: { access: "write" },
                camera: {},
            },
        ],
    );
});

test("refuse a sandbox entry with fields beyond its ability's own", () => {
    expect(Sandbox.safeParse({ camera: { reason: "scans receipts" } }).success).toBe(false);
});

test("match the hosts a sandbox's network ability lists, by name, by wildcard and on a listed port", () => {
    const sandbox = Sandbox.parse({
        network: { connect: ["api.stripe.com", "*.amazonaws.com", "localhost:8080"] },
    });
    const any = Sandbox.parse({ network: { connect: ["*"] } });

    // allow listed names on any port, wildcard subdomains, and a listed port alone
    expect([
        Sandbox.match(sandbox, { hostname: "api.stripe.com", port: 443 }),
        Sandbox.match(sandbox, { hostname: "API.stripe.com.", port: 443 }),
        Sandbox.match(sandbox, { hostname: "email.eu-central-1.amazonaws.com", port: 587 }),
        Sandbox.match(sandbox, { hostname: "amazonaws.com", port: 443 }),
        Sandbox.match(sandbox, { hostname: "localhost", port: 8080 }),
        Sandbox.match(sandbox, { hostname: "localhost", port: 9090 }),
        Sandbox.match(sandbox, { hostname: "example.com", port: 443 }),
        Sandbox.match(any, { hostname: "example.com", port: 443 }),
        Sandbox.match(any, { hostname: "127.0.0.1", port: 55432 }),
        Sandbox.match(Sandbox.parse({}), { hostname: "api.stripe.com", port: 443 }),
    ]).toEqual([
        "name",
        "name",
        "wildcard",
        undefined,
        "name",
        undefined,
        undefined,
        "wildcard",
        "wildcard",
        undefined,
    ]);
});
