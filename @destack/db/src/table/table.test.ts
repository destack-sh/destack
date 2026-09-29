import { expect, test } from "@destack/test";
import { alias, defineTable, Table, TABLE } from "./table.ts";
import { text } from "./column.ts";
import { primaryKey } from "./constraint.ts";

/** A table with a compound key. */
const membership = defineTable(
    "table_membership",
    {
        /** The group. */
        group: text("group").notNull(),
        /** The member. */
        member: text("member").notNull(),
        /** The member's role. */
        role: text("role").notNull(),
    },
    {
        constraints: (row) => [primaryKey({ columns: [row.group, row.member] })],
    },
);

test("key an alias by the properties of its source's compound key", () => {
    // read the key of the table and an alias
    const aliased = alias(membership, "table_membership_other");

    expect([membership[TABLE].key, aliased[TABLE].key]).toEqual([
        ["group", "member"],
        ["group", "member"],
    ]);
});

test("read the package declaring a table, and none for other values", () => {
    // read the stamped package of the table, a structural copy and other values
    const copy = { [Symbol.for("destack.table")]: membership[TABLE] };
    expect([
        Table.package(membership),
        Table.package(copy),
        Table.package({ package: membership[TABLE].package }),
        Table.package(undefined),
    ]).toEqual([membership[TABLE].package, membership[TABLE].package, undefined, undefined]);
});
