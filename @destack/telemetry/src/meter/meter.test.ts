import { Package } from "@destack/package";
import { expect, test } from "@destack/test";
import { defineMeter } from "../declare/meter.ts";

/** A package declaring a meter with a direction. */
const storage = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000d2",
    name: "@example/storage",
    version: "2026.9.0",
});

/** The operations the storage package serves, read or written. */
const operation = defineMeter(
    {
        name: "storage.operation",
        description: "Reads and writes.",
        aggregation: "sum",
        unit: "operation",
        attributes: { direction: ["read", "write"] },
    },
    { package: storage },
);

test("key a meter's events by their attribute values, refusing values the meter does not declare", () => {
    // key a declared value, and refuse a missing, an unknown and an undeclared attribute
    const refusal = (attributes: Readonly<Record<string, string>>) => {
        try {
            return operation.key(attributes);
        } catch (error) {
            if (!(error instanceof TypeError)) {
                throw error;
            }

            return error.message;
        }
    };
    expect([
        refusal({ direction: "write" }),
        refusal({}),
        refusal({ direction: "delete" }),
        refusal({ direction: "read", region: "eu" }),
    ]).toEqual([
        "package-01996ab0-0000-7000-8000-0000000000d2/storage.operation direction=write",
        'meter storage.operation takes attributes {"direction":["read","write"]}, not {}',
        'meter storage.operation takes attributes {"direction":["read","write"]}, not {"direction":"delete"}',
        'meter storage.operation takes attributes {"direction":["read","write"]}, not {"direction":"read","region":"eu"}',
    ]);
});
