import { expect, test } from "@destack/test";
import { Package } from "@destack/package";
import { alias, defineTable, TABLE } from "./table.ts";
import { text } from "./column.ts";

/** A table with a compound key. */
const membership = defineTable("table_membership", {
    /** The group. */
    group: text("group").primaryKey(),
    /** The member. */
    member: text("member").primaryKey(),
    /** The member's role. */
    role: text("role").notNull(),
});

test("key an alias by the properties of its source's compound key", () => {
    // read the key of the table and an alias
    const aliased = alias(membership, "table_membership_other");

    expect([membership[TABLE].key, aliased[TABLE].key]).toEqual([
        ["group", "member"],
        ["group", "member"],
    ]);
});

test("carry the declaring package under the shared package key", () => {
    expect(Package.declaring(membership)).toEqual(membership[TABLE].package);
});
