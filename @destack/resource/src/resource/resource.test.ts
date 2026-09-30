import { expect, test } from "@destack/test";
import { identifier, schema } from "@destack/schema";
import { Package } from "@destack/package";
import { ResourceContext } from "../context/index.ts";
import { Plan, Provider, Resource, defineResourceKind, type Step } from "./index.ts";

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
        spec: {},
    });
    const tasks = new Resource<string>(owner, {
        name: "tasks",
        kind: "database",
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
        action: "create",
        target: "table/note/column/priority",
        risk: "safe",
        detail: "add column priority",
    };
    const drop: Step = {
        action: "delete",
        target: "table/note/column/body",
        risk: "destructive",
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

test("validate declarations against their kind and spec", () => {
    const bucket = defineResourceKind("bucket", {
        spec: schema.object({ public: schema.boolean() }),
    });

    // accept a declaration of the kind and refuse another kind or spec
    const parse = (value: unknown) => bucket.description.safeParse(value).success;
    expect([
        parse({ name: "files", kind: "bucket", spec: { public: false } }),
        parse({ name: "files", kind: "vault", spec: { public: false } }),
        parse({ name: "files", kind: "bucket", spec: {} }),
    ]).toEqual([true, false, false]);
});

test("require reconciling from providers of kinds with a state, and whole capabilities from any", () => {
    const database = defineResourceKind("database", {
        spec: schema.object({}),
        state: schema.object({ tables: schema.array(schema.string()) }),
    });
    const bucket = defineResourceKind("bucket", { spec: schema.object({}) });
    const provision = async () => ({ reference: "memory:x" });
    const destroy = async () => {};

    // refuse a stateful kind's provider without reconciling, half a capability, and reconciling without a state
    // @ts-expect-error a provider of a kind with a state plans and applies
    const unreconciled: Provider<typeof database> = { kind: database, code: "memory" };
    // @ts-expect-error a provider provisions and destroys, or does neither
    const half: Provider<typeof bucket> = { kind: bucket, code: "memory", provision };
    const stateless: Provider<typeof bucket> = {
        kind: bucket,
        code: "memory",
        // @ts-expect-error a provider of a kind without a state reconciles nothing
        plan: async () => ({ steps: [] }),
    };

    // report each provider's capabilities
    const hosted: Provider<typeof bucket> = { kind: bucket, code: "memory", provision, destroy };
    expect(
        [unreconciled, half, stateless, hosted].map((provider) => [
            Provider.reconciles(provider),
            Provider.provisions(provider),
            Provider.copies(provider),
        ]),
    ).toEqual([
        [false, false, false],
        [false, false, false],
        [false, false, false],
        [false, true, false],
    ]);
});

test("read stored resources and desired states through their kind, refusing other kinds, invalid values and states of a stateless kind", () => {
    const database = defineResourceKind("database", {
        spec: schema.object({ tier: schema.enum(["zone", "global"]) }),
        state: schema.object({ tables: schema.array(schema.string()) }),
    });
    const bucket = defineResourceKind("bucket", { spec: schema.object({}) });
    const stored = {
        id: identifier("resource").parse("resource-01996ab0-0000-7000-8000-000000000001"),
        scope: identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
        kind: "database",
        spec: { tier: "zone" },
        reference: null,
    };
    const outcome = (read: () => unknown) => {
        try {
            return read();
        } catch (error) {
            return (error as Error).constructor.name;
        }
    };

    // read a database and its states, and refuse a bucket reading it, an invalid spec and states of a stateless kind
    expect([
        outcome(() => database.record(stored).spec),
        outcome(() => database.states([{ tables: ["note"] }])),
        outcome(() => bucket.record(stored)),
        outcome(() => database.record({ ...stored, spec: { tier: "planet" } })),
        outcome(() => database.states([{ tables: "note" }])),
        outcome(() => bucket.states([{}])),
        outcome(() => bucket.states([{ tables: [] }])),
    ]).toEqual([
        { tier: "zone" },
        [{ tables: ["note"] }],
        "TypeError",
        "ZodError",
        "ZodError",
        [],
        "TypeError",
    ]);
});
