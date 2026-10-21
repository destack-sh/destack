import { none, Policy } from "@destack/access";
import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { Expression } from "@destack/db";
import { PackageId, Package } from "@destack/package";
import { defineAdapter, definePackage, type ObjectDeclaration } from "../declare/package.ts";
import type { ProcedureRouter } from "../procedure/index.ts";
import { defineProcedure } from "../procedure/index.ts";
import { comparePackage, describePackage, packageSymbols } from "./package.ts";

/** The release under comparison. */
const RELEASE = "2026.9.0";

/** The package whose releases the entries declare. */
const PACKAGE = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-00000000c001"),
    name: "@destack/compare-fixture",
};

/** A release's manifest entry of a declaration, its description read back from JSON. */
const entry = (description: unknown, version: string) => ({
    description: schema.jsonObject().parse(JSON.parse(JSON.stringify(description))),
    symbol: {
        package: { ...PACKAGE, version },
        symbol: { module: "src/index.ts", name: "declared" },
    },
});

/** The module declaring the package's releases. */
const MODULE = { package: { ...PACKAGE, version: RELEASE } };

/** A query type of the package routing a set of procedures. */
const query = (procedures: ProcedureRouter): ObjectDeclaration => ({
    package: PACKAGE,
    name: "query",
    typeReference: { packageId: PACKAGE.id, type: "query" },
    hooks: {},
    procedures,
    shared: {},
});

/** A procedure without requirements. */
const procedure = (convert?: Readonly<Record<string, Readonly<Record<string, Expression>>>>) =>
    defineProcedure({
        package: Package.of(import.meta.destack.package),
        authentication: "public",
        permission: "anyone",
        audit: false,
        ...(convert === undefined ? {} : { convert }),
    });

test("plan a package's API changes between releases, converting narrowed inputs and refusing unconverted ones", () => {
    // describe an earlier release of the package's queries
    const before = describePackage(
        definePackage(
            {
                objects: {
                    query: query({
                        search: procedure()
                            .input(schema.object({ text: schema.string() }))
                            .output(
                                schema.object({
                                    hits: schema.array(schema.string()),
                                    total: schema.number(),
                                }),
                            ),
                        count: procedure().input(schema.object({ text: schema.string() })),
                        retired: procedure(),
                    }),
                },
            },
            MODULE,
        ),
    );

    // rename the search input and narrow the count input
    const release = (count: ReturnType<typeof procedure>) =>
        describePackage(
            definePackage(
                {
                    since: RELEASE,
                    objects: {
                        query: query({
                            search: procedure({ [RELEASE]: { query: Expression.column("text") } })
                                .input(schema.object({ query: schema.string() }))
                                .output(schema.object({ hits: schema.array(schema.string()) })),
                            count: count.input(schema.object({ text: schema.string().min(3) })),
                            suggest: procedure(),
                        }),
                    },
                },
                MODULE,
            ),
        );
    const converted = release(
        procedure({
            [RELEASE]: {
                text: Expression.coalesce(Expression.column("text"), Expression.literal("any")),
            },
        }),
    );
    const unconverted = release(procedure());
    const outcome = (after: ReturnType<typeof describePackage>) => {
        try {
            return comparePackage(entry(before, "2026.8.0"), entry(after, RELEASE)).steps;
        } catch (error) {
            // read the refusal's message
            if (!(error instanceof Error)) {
                throw new TypeError("comparing packages failed with a value that is no error", {
                    cause: error,
                });
            }

            return error.message;
        }
    };

    // plan the release's steps
    expect([outcome(before), outcome(converted), outcome(unconverted)]).toEqual([
        [],
        [
            {
                action: "update",
                target: "api",
                risk: "backward-incompatible",
                detail: "serve callers from 2026.9.0: earlier callers stay on their release",
            },
            {
                action: "convert",
                target: "api/procedure/query.count/input",
                risk: "fallible",
                detail: "convert values to 2026.9.0",
            },
            {
                action: "convert",
                target: "api/procedure/query.search/input",
                risk: "fallible",
                detail: "convert values to 2026.9.0",
            },
            {
                action: "update",
                target: "api/procedure/query.search/output",
                risk: "backward-incompatible",
                detail: "incompatible values: earlier readers stay on their release",
            },
            {
                action: "create",
                target: "api/procedure/query.suggest",
                risk: "safe",
                detail: "add procedure query.suggest",
            },
            {
                action: "delete",
                target: "api/procedure/query.retired",
                risk: "backward-incompatible",
                detail: "remove procedure query.retired: earlier callers stay on their release",
            },
        ],
        "api/procedure/query.count/input: declare a conversion for 2026.9.0",
    ]);
});

test("plan a package's adapter changes: adding is safe, removing or moving breaks callers", () => {
    // describe a release of the package's adapters
    const release = (adapters: Readonly<Record<string, `/${string}`>>) =>
        describePackage(
            definePackage(
                {
                    adapters: Object.fromEntries(
                        Object.entries(adapters).map(([name, path]) => [
                            name,
                            defineAdapter(name, { path }, MODULE),
                        ]),
                    ),
                },
                MODULE,
            ),
        );
    const before = release({ feed: "/feed", npm: "/npm", retired: "/retired" });
    const after = release({ feed: "/feed", npm: "/registry", archive: "/archive" });

    // plan the added, moved and removed adapters
    expect(comparePackage(entry(before, "2026.8.0"), entry(after, RELEASE)).steps).toEqual([
        {
            action: "create",
            target: "adapter/archive",
            risk: "safe",
            detail: "add adapter archive",
        },
        {
            action: "update",
            target: "adapter/npm",
            risk: "backward-incompatible",
            detail: "move adapter npm from /npm to /registry",
        },
        {
            action: "delete",
            target: "adapter/retired",
            risk: "backward-incompatible",
            detail: "remove adapter retired",
        },
    ]);
});

test("list a package's procedures as members the package serves", () => {
    // describe a package with a nested procedure
    const handle = definePackage(
        {
            objects: {
                query: query({
                    search: procedure().input(schema.object({ text: schema.string() })),
                    index: { rebuild: procedure() },
                }),
            },
        },
        MODULE,
    );
    const [rebuild, search] = describePackage(handle).api.procedures;

    expect(packageSymbols(handle)).toEqual([
        {
            relationships: [
                {
                    kind: "serves",
                    symbol: { kind: "procedure", name: "query.index.rebuild" },
                },
                {
                    kind: "serves",
                    symbol: { kind: "procedure", name: "query.search" },
                },
            ],
        },
        {
            member: { kind: "procedure", name: "query.index.rebuild", description: rebuild },
            relationships: [],
        },
        {
            member: { kind: "procedure", name: "query.search", description: search },
            relationships: [],
        },
    ]);
});

test("describe a package's routes, adapters, views, requested permissions and capabilities, with its own types' hooks", () => {
    // route the package's own query with hooks
    const own = { ...query({}), hooks: { create: ["authorize"] } };
    const user = new Policy(MODULE.package, { name: "user", permissions: { recipient: none() } });
    const handle = definePackage(
        {
            objects: { query: own },
            adapters: { npm: defineAdapter("npm", { path: "/npm" }, MODULE) },
            permissions: { installation: { universe: [user.permission("recipient")] } },
            capabilities: {
                push: {
                    reason: "Drop the browsers push services forgot",
                    optional: true,
                    permissions: { user: { home: [user.permission("recipient")] } },
                    features: [{ packageId: MODULE.package.id, name: "push" }],
                },
            },
        },
        MODULE,
    );

    expect(describePackage(handle)).toEqual({
        api: { name: handle.name, procedures: [] },
        objects: {
            query: { type: own.typeReference, hooks: { create: ["authorize"] } },
        },
        adapters: { npm: { path: "/npm" } },
        views: [],
        permissions: {
            installation: {
                universe: [{ packageId: MODULE.package.id, type: "user", name: "recipient" }],
            },
        },
        capabilities: {
            push: {
                reason: "Drop the browsers push services forgot",
                optional: true,
                permissions: {
                    user: {
                        home: [{ packageId: MODULE.package.id, type: "user", name: "recipient" }],
                    },
                },
                features: [{ packageId: MODULE.package.id, name: "push" }],
            },
        },
    });
});

test("refuse describing a procedure whose payload JSON Schema cannot describe, as a build does", () => {
    // describe a query whose input checks its name by code
    const refined = query({
        read: procedure().input(
            schema.object({ name: schema.string().refine((name) => name !== "latest") }),
        ),
    });

    expect(() => describePackage(definePackage({ objects: { read: refined } }, MODULE))).toThrow(
        "unsupported schema check: custom",
    );
});

test("refuse a package listing an object type another package defines, which that package alone serves", () => {
    const foreign: ObjectDeclaration = {
        ...query({}),
        package: { name: "@destack/board" },
        name: "card",
        typeReference: { packageId: "package-01996ab0-0000-7000-8000-00000000c002", type: "card" },
    };

    expect(() => definePackage({ objects: { card: foreign } }, MODULE)).toThrow(
        `${PACKAGE.name} lists object card of @destack/board, which @destack/board alone serves`,
    );
});
