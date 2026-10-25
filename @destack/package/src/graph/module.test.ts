import { testSimulation } from "@destack/test/world";
import { expect, test, refusal } from "@destack/test";
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
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("test");
    const declared = {
        kindPackageId: PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a2"),
        kind: "object",
        description: {},
    };
    const digest = await Digest.of(new Uint8Array(), crypto);
    const module = (at: Moniker): Module => ({
        path: "src/a.ts",
        digest,
        imports: [],
        exports: [{ name: "a", symbol: DEFINED, isTypeOnly: false }],
        declarations: [Declaration.at(at, { ...declared, name: "a" })],
        edges: [],
    });

    expect({
        encoded: (await Module.file(module(DEFINED), crypto)).digest,
        atModule: (await Module.file(module(MODULE), crypto)).digest,
        refused: await refusal(Module.file(module(ELSEWHERE), crypto)),
    }).toEqual({
        encoded: "55d4744af36f3fde66dd43c373c471a86359d5fad9047522a96edda64c41591a",
        atModule: "4cef01dbe758927b81ab590e962c369751b28e85b7c73ca675aec82bb2bc7a4e",
        refused: [
            "UNPROCESSABLE_CONTENT",
            "INSPECTION_INVALID",
            `src/a.ts declares ${ELSEWHERE}:object:a, which belongs to another module`,
        ],
    });
});
