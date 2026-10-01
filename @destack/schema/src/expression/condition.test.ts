import { expect, test } from "@destack/test";
import { Condition } from "./condition.ts";

test("build conditions, and read their columns, relations and terms", () => {
    const condition = Condition.all(
        Condition.eq("status", "open"),
        Condition.not(Condition.missing("due")),
        Condition.oneOf("priority", ["high", "low"]),
        Condition.exists("assignee", Condition.gt("level", Condition.parameter("level"))),
    );

    // list what the condition reads
    expect([
        [...Condition.columns(condition)],
        Condition.relations(condition).map((relation) => relation.via),
        Condition.terms(condition),
    ]).toEqual([["status", "due", "priority"], ["assignee"], 8]);

    // rename columns, leaving relations to their own rows
    expect(Condition.rename(condition, (column) => `t_${column}`)).toEqual(
        Condition.all(
            Condition.eq("t_status", "open"),
            Condition.not(Condition.missing("t_due")),
            Condition.oneOf("t_priority", ["high", "low"]),
            Condition.exists("assignee", Condition.gt("level", Condition.parameter("level"))),
        ),
    );

    // round-trip through the schema
    expect(Condition.schema.parse(JSON.parse(JSON.stringify(condition)))).toEqual(condition);
});
