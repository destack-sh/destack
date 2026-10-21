import { expect, test } from "@destack/test";
import { Sandbox } from "./sandbox.ts";

test("merge sandboxes into every ability of each, uniting host and command lists and taking writing over reading", () => {
    // join an indexer's base sandbox with what two of its capabilities unlock
    const base = Sandbox.parse({
        process: {},
        network: { connect: ["api.github.com"] },
        run: { commands: ["git"] },
        fs: { access: "read" },
    });
    const importing = Sandbox.parse({
        network: { connect: ["gmail.googleapis.com", "api.github.com"] },
    });
    const syncing = Sandbox.parse({ fs: { access: "write" }, camera: {} });

    expect([Sandbox.merge(), Sandbox.merge(base), Sandbox.merge(base, importing, syncing)]).toEqual(
        [
            {},
            base,
            {
                process: {},
                network: { connect: ["api.github.com", "gmail.googleapis.com"] },
                run: { commands: ["git"] },
                fs: { access: "write" },
                camera: {},
            },
        ],
    );
});

test("refuse a sandbox entry with fields beyond its ability's own", () => {
    expect(Sandbox.safeParse({ camera: { reason: "scans receipts" } }).success).toBe(false);
});
