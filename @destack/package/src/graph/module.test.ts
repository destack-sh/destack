import { expect, test } from "@destack/test";
import { Digest } from "@destack/schema";
import { PackageId } from "../definition/package.ts";
import { Declaration } from "./declaration.ts";
import { Module } from "./module.ts";
import { Moniker } from "./moniker.ts";

/** The symbol the module defines. */
const DEFINED = Moniker.of({
    packageId: "package-01996ab0-0000-7000-8000-0000000000a1",
    module: "src/a.ts",
    name: "a",
});

/** The module itself, where module-level declarations such as tests sit. */
const MODULE = Moniker.of({
    packageId: "package-01996ab0-0000-7000-8000-0000000000a1",
    module: "src/a.ts",
});

/** A symbol of another module. */
const ELSEWHERE = Moniker.of({
    packageId: "package-01996ab0-0000-7000-8000-0000000000a1",
    module: "src/b.ts",
    name: "b",
});

test("encode a module whose declarations sit in it, at itself or its symbols, refusing one in another module", async () => {
    // declare an object at the module's symbol, one at the module, and one at another module's symbol
    const declared = {
        kindPackageId: PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a2"),
        kind: "object",
        description: {},
    };
    const digest = await Digest.of(new Uint8Array());
    const module = (at: Moniker): Module => ({
        path: "src/a.ts",
        digest,
        imports: [],
        exports: [{ name: "a", symbol: DEFINED, isTypeOnly: false }],
        declarations: [Declaration.at(at, { ...declared, name: "a" })],
        edges: [],
    });

    expect({
        encoded: (await Module.file(module(DEFINED))).bytes.byteLength > 0,
        atModule: (await Module.file(module(MODULE))).bytes.byteLength > 0,
        refused: await Module.file(module(ELSEWHERE)).catch((error: unknown) => String(error)),
    }).toEqual({
        encoded: true,
        atModule: true,
        refused: `TypeError: src/a.ts declares ${ELSEWHERE}:object:a in another module`,
    });
});
