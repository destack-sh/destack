import { expect, test } from "@destack/test";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { ResourceDeclaration } from "./declaration.ts";

/** The package declaring the example resource. */
const owner = Package.parse({
    id: "package-01996ab0-0000-7000-8000-000000000001",
    name: "@example/billing",
    version: "2026.9.0",
});

test("declare a resource's create input with the stack's options", () => {
    const api = new ResourceDeclaration(owner, { name: "api", kind: "api", spec: {} });
    const definition = {
        definitionPackageId: owner.id,
        definitionVersion: "2026.9.0",
        definitionName: "api",
        spec: {},
    };

    expect(api.declare({ retention: "delete" })).toEqual({ ...definition, retention: "delete" });
});
