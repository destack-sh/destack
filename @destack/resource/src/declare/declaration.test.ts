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

/** The connection a stack lends the example API from. */
const connection = schema
    .identifier("connection")
    .parse("connection-01996ab0-0000-7000-8000-000000000002");

test("declare a resource's create input, and a lent one's from a connection", () => {
    const api = new ResourceDeclaration(owner, { name: "api", kind: "https", spec: {} });
    const definition = {
        definitionPackageId: owner.id,
        definitionVersion: "2026.9.0",
        definitionName: "api",
        spec: {},
    };

    expect([api.declare({ retention: "delete" }), api.declare({ connection })]).toEqual([
        { ...definition, retention: "delete" },
        { ...definition, connection },
    ]);
});
