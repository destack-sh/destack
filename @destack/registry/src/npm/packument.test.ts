import type { JsonObject } from "@destack/schema";
import { expect, test } from "@destack/test";
import { NpmPackument } from "./packument.ts";

/** The address of a release's tarball at the fixture forge. */
function tarball(version: string): string {
    return `https://forge.test/.npm/@example/notes/-/notes-${version}.tgz`;
}

test("list a package's releases in release order with their install fields, and its tags at listed releases", () => {
    // describe a deprecated first release, a dev build and the release two tags point at
    const release = (version: string, manifest: JsonObject = {}) => ({
        version,
        listed: { manifest, integrity: `sha512-${version}` },
        tarball: tarball(version),
    });
    const packument = NpmPackument.of(
        "@example/notes",
        { stable: "2026.11.3", latest: "2026.11.3", gone: "2026.10.0" },
        [
            release("2026.11.3", {
                dependencies: { "@destack/object": "2026.11.2" },
                bin: { notes: "./src/main.ts" },
            }),
            { ...release("2026.9.0"), deprecation: "use 2026.11" },
        ],
    );
    expect(packument).toEqual({
        name: "@example/notes",
        "dist-tags": { latest: "2026.11.3", stable: "2026.11.3" },
        versions: {
            "2026.9.0": {
                name: "@example/notes",
                version: "2026.9.0",
                dist: { tarball: tarball("2026.9.0"), integrity: "sha512-2026.9.0" },
                deprecated: "use 2026.11",
            },
            "2026.11.3": {
                name: "@example/notes",
                version: "2026.11.3",
                dependencies: { "@destack/object": "2026.11.2" },
                bin: { notes: "./src/main.ts" },
                dist: { tarball: tarball("2026.11.3"), integrity: "sha512-2026.11.3" },
            },
        },
    });
    expect(Object.keys(packument["versions"] ?? {})).toEqual(["2026.9.0", "2026.11.3"]);
});
