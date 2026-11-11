import { expect, test, refusal } from "@destack/test";
import { defineKind, PackageVersion } from "@destack/package";
import { schema } from "@destack/schema";
import { ResourceDeclaration } from "../declare/index.ts";
import { ResourceContext } from "./context.ts";

/** The package declaring the example resources. */
const owner = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-000000000001",
    name: "@example/notes",
    version: "2026.9.0",
});

/** The kind of the example resources. */
const KIND = defineKind(
    { name: "database", provisioned: { spec: schema.object({}) } },
    { package: owner },
);

test("bind one client per declaration and refuse missing or repeated bindings", async () => {
    const notes = new ResourceDeclaration<string>(owner, KIND, {
        name: "notes",
        spec: {},
    });
    const tasks = new ResourceDeclaration<string>(owner, KIND, {
        name: "tasks",
        spec: {},
    });
    const context = new ResourceContext().bind(notes, "notes client");

    // return the bound client through the declaration
    expect(notes.get(context)).toBe("notes client");

    // refuse an unbound declaration and a second binding
    expect(await refusal(() => tasks.get(context))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "BINDING_BROKEN",
        "resource is not bound: 'tasks'",
    ]);
    expect(await refusal(() => context.bind(notes, "other client"))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "BINDING_BROKEN",
        "resource already bound: 'notes'",
    ]);
});
