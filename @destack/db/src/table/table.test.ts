import { expect, test } from "@destack/test";
import { alias, defineTable, TABLE } from "./table.ts";
import { text } from "./column.ts";
import { primaryKey } from "./constraint.ts";

/** A table keyed by a compound key constraint. */
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
    // read the key of the table and of an alias of it
    const aliased = alias(membership, "table_membership_other");

    expect([membership[TABLE].key, aliased[TABLE].key]).toEqual([
        ["group", "member"],
        ["group", "member"],
    ]);
});
