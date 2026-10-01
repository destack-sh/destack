import { expect, test } from "@destack/test";
import { PackageError } from "../error/error.ts";
import { ModuleMetadata } from "./metadata.ts";

test("require the module metadata the transform passes to declaration constructors", () => {
    // accept the metadata of a package module
    const module = {
        package: {
            id: "package-01996ab0-0000-7000-8000-000000000001",
            name: "@example/notes",
            version: "2026.10.0",
        },
    };
    expect(ModuleMetadata.require(module as ModuleMetadata, "defineVault")).toEqual(module);

    // refuse a call the transform did not stamp, and metadata of no release
    expect(() => ModuleMetadata.require(undefined, "defineVault")).toThrow(
        new PackageError(
            "INVALID_DEFINITION",
            "defineVault requires the Destack module transform to supply its package",
        ),
    );
    const refused = ModuleMetadata.safeParse({ package: { ...module.package, version: "1" } });
    expect(refused.error?.issues.map((issue) => issue.path.join("."))).toEqual(["package.version"]);
});
