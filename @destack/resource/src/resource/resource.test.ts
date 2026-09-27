import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { Package } from "@destack/package";
import { ResourceContext } from "../context/index.ts";
import { Plan, Resource, defineResourceSchema, type Step } from "./index.ts";

/** The package declaring the example resources. */
const owner = Package.parse({
    id: "package-01996ab0-0000-7000-8000-000000000001",
    name: "@example/notes",
    version: "2026.9.0",
});

test("bind one client per declaration and refuse missing or repeated bindings", () => {
    const notes = new Resource<string>(owner, {
        name: "notes",
        kind: "database",
        version: 1,
        spec: {},
    });
    const tasks = new Resource<string>(owner, {
        name: "tasks",
        kind: "database",
        version: 1,
        spec: {},
    });
    const context = new ResourceContext().bind(notes, "notes client");

    // return the bound client through the declaration
    expect(notes.get(context)).toBe("notes client");

    // refuse an unbound declaration and a second binding
    expect(() => tasks.get(context)).toThrow("resource is not bound: tasks");
    expect(() => context.bind(notes, "other client")).toThrow("resource already bound: notes");
});

test("classify a plan by its most consequential step and digest its reviewed steps", async () => {
    const add: Step = {
        kind: "addColumn",
        risk: "safe",
        target: "note",
        detail: "add column priority",
    };
    const drop: Step = {
        kind: "dropColumn",
        risk: "destructive",
        target: "note",
        detail: "drop column body",
    };

    // rank plans by their highest risk, safe when empty
    expect([Plan.classify({ steps: [] }), Plan.classify({ steps: [add, drop] })]).toEqual([
        "safe",
        "destructive",
    ]);

    // digest the same steps equally and different steps differently
    const first = await Plan.digest({ steps: [add, drop] });
    expect(await Plan.digest({ steps: [add, drop] })).toBe(first);
    expect(await Plan.digest({ steps: [drop, add] })).not.toBe(first);
});

test("validate declarations against their kind, version and spec", () => {
    const Bucket = defineResourceSchema("bucket", 1, schema.object({ public: schema.boolean() }));

    // accept a declaration of the kind and refuse another kind or spec
    expect(
        Bucket.parse({ name: "files", kind: "bucket", version: 1, spec: { public: false } }),
    ).toEqual({
        name: "files",
        kind: "bucket",
        version: 1,
        spec: { public: false },
    });
    expect(
        Bucket.safeParse({ name: "files", kind: "vault", version: 1, spec: { public: false } })
            .success,
    ).toBe(false);
    expect(Bucket.safeParse({ name: "files", kind: "bucket", version: 1, spec: {} }).success).toBe(
        false,
    );
});
