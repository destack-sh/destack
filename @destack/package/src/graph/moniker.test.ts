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

test("read the module path of a module, a symbol, a member and declarations at a module and at a symbol", () => {
    const at = "package-01996ab0-0000-7000-8000-0000000000a1/src/note.ts";

    expect(
        [at, `${at}#Note`, `${at}#Note.rename`, `${at}:test:note`, `${at}#Note:object`].map(
            (moniker) => Moniker.module(Moniker.parse(moniker)),
        ),
    ).toEqual(["src/note.ts", "src/note.ts", "src/note.ts", "src/note.ts", "src/note.ts"]);
});

test("read the symbol name of a module, a symbol, a member and declarations at a module and at a symbol", () => {
    const at = "package-01996ab0-0000-7000-8000-0000000000a1/src/note.ts";

    expect(
        [at, `${at}#Note`, `${at}#Note.rename`, `${at}:test:note`, `${at}#Note:object`].map(
            (moniker) => Moniker.name(Moniker.parse(moniker)),
        ),
    ).toEqual([undefined, "Note", "Note.rename", undefined, "Note"]);
});
