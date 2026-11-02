import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { defineMeter } from "../declare/meter.ts";
import { compareMeter, describeMeter } from "./meter.ts";

/** A package declaring a requests meter. */
const storage = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000d1",
    name: "@example/storage",
    version: "2026.9.0",
});

/** The requests the storage package serves. */
const requests = defineMeter(
    { name: "api.requests", description: "API requests.", aggregation: "sum", unit: "request" },
    { package: storage },
);

/** Read a description back from JSON, as a release keeps it. */
function readBack(description: unknown) {
    return schema.jsonObject().parse(JSON.parse(JSON.stringify(description)));
}

/** Describe the requests meter counting another way or in another unit. */
function counting(aggregation: "sum" | "max", unit: string) {
    return readBack(
        describeMeter(
            defineMeter({ ...requests.definition, aggregation, unit }, { package: storage }),
        ),
    );
}

test("plan a meter's change between releases: keep its aggregation and unit", () => {
    const outcome = (aggregation: "sum" | "max", unit: string) => {
        try {
            return compareMeter(
                { description: counting("sum", "request"), symbol: { package: storage } },
                {
                    description: counting(aggregation, unit),
                    symbol: { package: { ...storage, version: "2026.10.0" } },
                },
            ).steps;
        } catch (error) {
            if (!(error instanceof Error)) {
                throw error;
            }

            return error.message;
        }
    };

    // keep an unchanged meter, and refuse counting another way or in another unit
    expect([outcome("sum", "request"), outcome("max", "call")]).toEqual([
        [],
        "meter/api.requests: keep aggregation sum, or declare a meter of aggregation max; meter/api.requests: keep unit request, or declare a meter of unit call",
    ]);
});
