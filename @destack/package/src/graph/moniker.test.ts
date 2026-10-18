import { expect, test } from "@destack/test";
import { Moniker } from "./moniker.ts";

test("read the package a moniker starts with, an identifier or an npm name with its scope", () => {
    // join monikers of a module, a symbol and a member, for a package and for npm packages
    const monikers = [
        Moniker.of({
            packageId: "package-01996ab0-0000-7000-8000-0000000000a1",
            module: "src/a.ts",
        }),
        Moniker.of({ packageId: "npm:@scope/name", module: "dist/b.js", name: "b" }),
        Moniker.of({ packageId: "npm:plain", module: "c.js", name: "c", member: "d" }),
    ];

    expect(monikers.map((moniker) => Moniker.packageId(moniker))).toEqual([
        "package-01996ab0-0000-7000-8000-0000000000a1",
        "npm:@scope/name",
        "npm:plain",
    ]);
});
