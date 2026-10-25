import { expect, test, refusal } from "@destack/test";
import { Package } from "@destack/package";
import { ResourceDeclaration } from "../declare/index.ts";
import { ResourceContext } from "./context.ts";

/** The package declaring the example resources. */
const owner = Package.parse({
    id: "package-01996ab0-0000-7000-8000-000000000001",
    name: "@example/notes",
    version: "2026.9.0",
});

test("bind one client per declaration and refuse missing or repeated bindings", async () => {
    const notes = new ResourceDeclaration<string>(owner, {
        name: "notes",
        kind: "database",
        spec: {},
    });
    const tasks = new ResourceDeclaration<string>(owner, {
        name: "tasks",
        kind: "database",
        spec: {},
    });
    const context = new ResourceContext().bind(notes, "notes client");

    // return the bound client through the declaration
    expect(notes.get(context)).toBe("notes client");

    // refuse an unbound declaration and a second binding
    expect(await refusal(() => tasks.get(context))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "BINDING_MISSING",
        "resource is not bound: tasks",
    ]);
    expect(await refusal(() => context.bind(notes, "other client"))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "ALREADY_BOUND",
        "resource already bound: notes",
    ]);
});
