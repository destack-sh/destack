import { expect, test } from "@destack/test";
import { PackageJson } from "./description.ts";

test("list a package's runtime dependencies, and decide which it runs without", () => {
    // declare a dependency, a peer, an optional dependency and an optional peer
    const declared = {
        dependencies: { zod: "4.6.5" },
        peerDependencies: { "solid-js": "2.0.0", vite: "8.0.0" },
        optionalDependencies: { sharp: "0.34.0" },
        peerDependenciesMeta: { vite: { optional: true } },
    };

    expect({
        names: PackageJson.runtimeDependencies(declared),
        optional: ["zod", "solid-js", "vite", "sharp"].map((name) =>
            PackageJson.isOptional(declared, name),
        ),
    }).toEqual({
        names: ["zod", "solid-js", "vite", "sharp"],
        optional: [false, false, true, true],
    });
});
