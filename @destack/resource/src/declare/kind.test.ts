import { expect, test } from "@destack/test";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { defineResourceKind } from "./kind.ts";

/** The package declaring the example kinds. */
const owner = Package.parse({
    id: "package-01996ab0-0000-7000-8000-000000000001",
    name: "@example/storage",
    version: "2026.9.0",
});

test("tell a kind reconciling toward a desired state from one without", () => {
    // declare a kind with a desired state and one without
    const module = { package: owner };
    const stateful = defineResourceKind(
        "table",
        { spec: schema.object({}), state: schema.object({ columns: schema.number() }) },
        module,
    );
    const stateless = defineResourceKind("link", { spec: schema.object({}) }, module);

    expect([stateful.isStateful, stateless.isStateful]).toEqual([true, false]);
});
