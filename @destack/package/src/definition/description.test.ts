import { expect, test } from "@destack/test";
import { PackageJson } from "./description.ts";

test("list a package's runtime dependencies and peers, and decide which it runs without", () => {
    // declare a dependency, a peer, an optional dependency, an optional peer, and one its metadata alone lists
    const declared = {
        dependencies: { zod: "4.6.5" },
        peerDependencies: { "solid-js": "2.0.0", vite: "8.0.0", zod: "4.6.5" },
        optionalDependencies: { sharp: "0.34.0" },
        peerDependenciesMeta: { vite: { optional: true }, typescript: { optional: true } },
    };

    // count a peer the package also requires as a dependency only
    expect({
        names: PackageJson.runtimeDependencies(declared),
        peers: PackageJson.peers(declared),
        optional: ["zod", "solid-js", "vite", "sharp", "typescript"].map((name) =>
            PackageJson.isOptional(declared, name),
        ),
    }).toEqual({
        names: ["zod", "sharp", "solid-js", "vite", "typescript"],
        peers: ["solid-js", "vite", "typescript"],
        optional: [false, false, true, true, true],
    });
});

test("release a package file's workspace ranges at linked versions, the default tag where none links", () => {
    const authored = {
        name: "@acme/notes",
        dependencies: { "@destack/object": "workspace:*", zod: "4.1.0" },
        peerDependencies: { "@destack/view": "workspace:^" },
        devDependencies: { "@destack/test": "workspace:*" },
    };
    expect(PackageJson.release(authored, { "@destack/object": "2026.11.2" })).toEqual({
        name: "@acme/notes",
        dependencies: { "@destack/object": "2026.11.2", zod: "4.1.0" },
        peerDependencies: { "@destack/view": "latest" },
        devDependencies: { "@destack/test": "latest" },
    });
});
