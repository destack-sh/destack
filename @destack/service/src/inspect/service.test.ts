import { expect, test } from "@destack/test";
import { Expression } from "@destack/schema/expression";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { defineService } from "../declare/service.ts";
import { defineProcedure } from "../procedure/index.ts";
import { compareService, describeService } from "./service.ts";

/** The release under comparison. */
const RELEASE = "2026.9.0";

/** The package whose releases the entries declare. */
const PACKAGE = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-00000000c001"),
    name: "@destack/compare-fixture",
};

/** A release's manifest entry of a declaration, its description read back from JSON. */
const entry = (description: unknown, version: string) => ({
    description: schema
        .record(schema.string(), schema.json())
        .parse(JSON.parse(JSON.stringify(description))),
    symbol: {
        package: { ...PACKAGE, version },
        symbol: { module: "src/index.ts", name: "declared" },
    },
});

/** A procedure without requirements. */
const procedure = (convert?: Readonly<Record<string, Readonly<Record<string, Expression>>>>) =>
    defineProcedure({
        authentication: "public",
        permission: null,
        audit: false,
        ...(convert === undefined ? {} : { convert }),
    });

test("plan a service's changes between releases, converting narrowed inputs and refusing unconverted ones", () => {
    // describe an earlier release of the search service
    const before = describeService(
        defineService("search", {
            search: procedure()
                .input(schema.object({ text: schema.string() }))
                .output(
                    schema.object({ hits: schema.array(schema.string()), total: schema.number() }),
                ),
            count: procedure().input(schema.object({ text: schema.string() })),
            retired: procedure(),
        }),
    );

    // rename the search input and narrow the count input with conversions, drop the total earlier callers read, and add suggestions
    const release = (count: ReturnType<typeof procedure>) =>
        describeService(
            defineService("search", {
                since: RELEASE,
                search: procedure({ [RELEASE]: { query: Expression.column("text") } })
                    .input(schema.object({ query: schema.string() }))
                    .output(schema.object({ hits: schema.array(schema.string()) })),
                count: count.input(schema.object({ text: schema.string().min(3) })),
                suggest: procedure(),
            }),
        );
    const converted = release(
        procedure({
            [RELEASE]: {
                text: Expression.coalesce(Expression.column("text"), Expression.literal("any")),
            },
        }),
    );
    const unconverted = release(procedure());
    const outcome = (after: ReturnType<typeof describeService>) => {
        try {
            return compareService(entry(before, "2026.8.0"), entry(after, RELEASE)).steps;
        } catch (error) {
            return (error as Error).message;
        }
    };

    // plan the release's steps, and refuse the narrowing without a conversion
    expect([outcome(before), outcome(converted), outcome(unconverted)]).toEqual([
        [],
        [
            {
                action: "update",
                target: "service/search",
                risk: "backward-incompatible",
                detail: "serve callers from 2026.9.0: earlier callers keep their release",
            },
            {
                action: "convert",
                target: "service/search/procedure/count/input",
                risk: "data-dependent",
                detail: "convert values to 2026.9.0",
            },
            {
                action: "convert",
                target: "service/search/procedure/search/input",
                risk: "data-dependent",
                detail: "convert values to 2026.9.0",
            },
            {
                action: "update",
                target: "service/search/procedure/search/output",
                risk: "backward-incompatible",
                detail: "incompatible values: earlier readers keep their release",
            },
            {
                action: "create",
                target: "service/search/procedure/suggest",
                risk: "safe",
                detail: "add procedure suggest",
            },
            {
                action: "delete",
                target: "service/search/procedure/retired",
                risk: "backward-incompatible",
                detail: "remove procedure retired: earlier callers keep their release",
            },
        ],
        "service/search/procedure/count/input: declare a conversion for 2026.9.0",
    ]);
});
