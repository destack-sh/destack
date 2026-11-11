import { PackageVersion } from "@destack/package";
import { testSimulation } from "@destack/object/test";
import { Digest } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Registry } from "./registry.ts";

/** The finance package at a version. */
function finance(version: string) {
    return PackageVersion.parse({
        id: "package-01a10720-c4d6-7559-816b-99f193c28c3c",
        name: "@destack/finance",
        version,
    });
}

test("answer a shipped version exactly and a shipped release by its newest dev build, and ask the forges for any other release or tag", async () => {
    // ship two dev builds of one release beside forges that refuse every call, naming the release
    const environment = testSimulation().environment("registry");
    const registry = new Registry(
        (owner) => Promise.reject(new TypeError(`asked the forge of ${JSON.stringify(owner)}`)),
        environment,
        [
            { package: finance("2026.9.0-dev.1"), manifest: Digest.parse("1".repeat(64)) },
            { package: finance("2026.9.0-dev.2"), manifest: Digest.parse("2".repeat(64)) },
        ],
    );
    const { id } = finance("2026.9.0");
    const asked = `asked the forge of ${JSON.stringify({ package: id })}`;

    // find the shipped ones without a forge, and ask the publisher's forge for the others
    expect({
        shipped: await registry.find(id, "2026.9.0"),
        exact: await registry.find(id, "2026.9.0-dev.1"),
        other: await registry.find(id, "2026.10.0").catch((error: unknown) => String(error)),
        tag: await registry.find(id, "latest").catch((error: unknown) => String(error)),
    }).toEqual({
        shipped: "2".repeat(64),
        exact: "1".repeat(64),
        other: `TypeError: ${asked}`,
        tag: `TypeError: ${asked}`,
    });
});
