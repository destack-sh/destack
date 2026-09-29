import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
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
    const connect = async () => "client";
    const provision = async () => ({ reference: "memory:x" });
    const destroy = async () => {};

    // refuse a stateful kind's provider without reconciling, half a capability, and reconciling without a state
    // @ts-expect-error a provider of a kind with a state plans and applies
    const unreconciled: Provider<string, typeof database> = {
        kind: "database",
        code: "memory",
        connect,
    };
    // @ts-expect-error a provider provisions and destroys, or does neither
    const half: Provider<string, typeof bucket> = {
        kind: "bucket",
        code: "memory",
        connect,
        provision,
    };
    const stateless: Provider<string, typeof bucket> = {
        kind: "bucket",
        code: "memory",
        connect,
        // @ts-expect-error a provider of a kind without a state reconciles nothing
        plan: async () => ({ steps: [] }),
    };

    // report each provider's capabilities
    const hosted: Provider<string, typeof bucket> = {
        kind: "bucket",
        code: "memory",
        connect,
        provision,
        destroy,
    };
    const capabilities = (provider: Provider) => [
        Provider.reconciles(provider),
        Provider.provisions(provider),
        Provider.copies(provider),
    ];
    expect(
        [unreconciled, half, stateless, hosted].map((provider) =>
            capabilities(provider as Provider),
        ),
    ).toEqual([
        [false, false, false],
        [false, false, false],
        [false, false, false],
        [false, true, false],
    ]);
});
