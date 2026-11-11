import { expect, test } from "@destack/test";
import { defineKind, PackageVersion } from "@destack/package";
import { schema } from "@destack/schema";
import { ResourceDeclaration } from "./declaration.ts";

/** The package declaring the example resource. */
const owner = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-000000000001",
    name: "@example/billing",
    version: "2026.9.0",
});

/** The kind of the example resources. */
const KIND = defineKind(
    { name: "api", provisioned: { spec: schema.object({}) } },
    { package: owner },
);

test("declare a resource's create input with the stack's options", () => {
    const api = new ResourceDeclaration(owner, KIND, { name: "api", spec: {} });
    const definition = {
        definitionPackageId: owner.id,
        definitionVersion: "2026.9.0",
        definitionName: "api",
        spec: {},
    };

    expect(api.provision({ retention: "delete" })).toEqual({ ...definition, retention: "delete" });
});
