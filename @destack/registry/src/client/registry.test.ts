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

test("answer a shipped release by its version or prerelease, and ask the forges for any other release or tag", async () => {
    // ship a prerelease beside forges that refuse every call
    const environment = testSimulation().environment("registry");
    const prerelease = "2026.9.0-dev.20260912100000.0123456789ab";
    const registry = new Registry(
        (owner) => Promise.reject(new TypeError(`asked the forge of ${JSON.stringify(owner)}`)),
        environment,
        [{ package: finance(prerelease), lock: Digest.parse("1".repeat(64)) }],
    );
    const { id } = finance("2026.9.0");
    const asked = `asked the forge of ${JSON.stringify({ package: id })}`;

    // find the shipped one without a forge
    expect({
        shipped: await registry.find(id, "2026.9.0"),
        exact: await registry.find(id, prerelease),
        other: await registry.find(id, "2026.10.0").catch((error: unknown) => String(error)),
        tag: await registry.find(id, "latest").catch((error: unknown) => String(error)),
    }).toEqual({
        shipped: "1".repeat(64),
        exact: "1".repeat(64),
        other: `TypeError: ${asked}`,
        tag: `TypeError: ${asked}`,
    });
});
