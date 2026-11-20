import { PackageVersion } from "@destack/package";
import { expect, test } from "@destack/test";
import { defineMeter } from "../declare/meter.ts";
import { t } from "@destack/locale";

/** A package declaring a meter with a direction. */
const storage = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000d2",
    name: "@example/storage",
    version: "2026.9.0",
});

/** The operations the storage package serves, read or written. */
const operation = defineMeter(
    {
        name: "storage.operation",
        description: t`Reads and writes.`,
        aggregation: "sum",
        unit: "operation",
        attributes: { direction: ["read", "write"] },
    },
    { package: storage },
);

/** Key the operation meter's attributes, or read its refusal. */
function keyOrRefusal(attributes: Readonly<Record<string, string>>) {
    try {
        return operation.key(attributes);
    } catch (error) {
        if (!(error instanceof TypeError)) {
            throw error;
        }

        return error.message;
    }
}

test("key a meter's events by their attribute values, refusing values the meter does not declare", () => {
    // key a declared value, and refuse a missing, an unknown and an undeclared attribute
    expect([
        keyOrRefusal({ direction: "write" }),
        keyOrRefusal({}),
        keyOrRefusal({ direction: "delete" }),
        keyOrRefusal({ direction: "read", region: "eu" }),
    ]).toEqual([
        "package-01996ab0-0000-7000-8000-0000000000d2/storage.operation direction=write",
        'meter \'storage.operation\' takes attributes {"direction":["read","write"]}, not {}',
        'meter \'storage.operation\' takes attributes {"direction":["read","write"]}, not {"direction":"delete"}',
        'meter \'storage.operation\' takes attributes {"direction":["read","write"]}, not {"direction":"read","region":"eu"}',
    ]);
});
