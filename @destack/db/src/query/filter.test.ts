import { expect, test } from "@destack/test";
import { DatabaseError } from "../error/error.ts";
import { Condition } from "./condition.ts";
import { Filter } from "./filter.ts";

test("read comparisons, named values and wildcards into conditions", () => {
    // compare by equality, order, containment, null and patterns
    const options = { values: { me: "user-1" } };
    expect(
        [
            "status = open",
            "assignee = @me",
            'title = "Ship it"',
            "due < 1760054400000",
            "priority >= 2",
            "status != done",
            "title:plan",
            "deletedAt = null",
            "deletedAt != null",
            "title = plan*",
            "isPinned = true",
        ].map((filter) => Filter.parse(filter, options)),
    ).toEqual([
        { status: "open" },
        { assignee: "user-1" },
        { title: "Ship it" },
        { due: { lt: 1760054400000 } },
        { priority: { gte: 2 } },
        { status: { ne: "done" } },
        { title: { ilike: "%plan%" } },
        { deletedAt: { isNull: true } },
        { deletedAt: { isNotNull: true } },
        { title: { like: "plan%" } },
        { isPinned: true },
    ]);
});

test("bind AND tighter than OR, join adjacent restrictions with AND, and negate with NOT or minus", () => {
    // read SQL's precedence, adjacency, negations, parentheses and relation paths
    expect(
        [
            "status = open AND priority = high OR due < 5",
            "status = open priority = high",
            "NOT status = done",
            "-labels:blocked",
            "(status = open OR status = blocked) AND assignee = bob",
            "project.name = Launch",
        ].map((filter) => Filter.parse(filter)),
    ).toEqual([
        { OR: [{ AND: [{ status: "open" }, { priority: "high" }] }, { due: { lt: 5 } }] },
        { AND: [{ status: "open" }, { priority: "high" }] },
        { NOT: { status: "done" } },
        { NOT: { labels: { ilike: "%blocked%" } } },
        { AND: [{ OR: [{ status: "open" }, { status: "blocked" }] }, { assignee: "bob" }] },
        { project: { name: "Launch" } },
    ]);
});

/** Convert a date literal for the due field. */
function convert(path: readonly string[], literal: string | number | boolean) {
    return path[0] === "due" && typeof literal === "string" ? Date.parse(literal) : literal;
}

test("escape pattern characters and convert literals for their fields", () => {
    // escape %, _ and backslashes, and convert a date for a time field
    expect([Filter.parse("title:50%_off"), Filter.parse("due < 2026-10-10", { convert })]).toEqual([
        { title: { ilike: "%50\\%\\_off%" } },
        { due: { lt: Date.parse("2026-10-10") } },
    ]);
});

test("produce conditions the condition schema accepts", () => {
    // validate a combined filter's condition
    const condition = Filter.parse('status = open AND (title:"q4 plan" OR -priority < 2)');
    expect(Condition.schema.parse(condition)).toEqual(condition);
});

test.each([
    ["status = ", "expected a value at 9 in filter status = "],
    ["= open", "expected a field at 0 in filter = open"],
    ["owner = @nobody", "unknown value @nobody at 8 in filter owner = @nobody"],
    ["secret = x", "unknown field secret at 0 in filter secret = x"],
    ["(status = open", "expected ) at 14 in filter (status = open"],
    ['title = "open', 'unclosed text at 8 in filter title = "open'],
    ["status open", "expected a comparison after status at 7 in filter status open"],
    ["status < null", "null compares only with = or != at 7 in filter status < null"],
])("refuse the filter %s", (filter, message) => {
    // refuse with the position of the failure
    expect(() => Filter.parse(filter, { fields: new Set(["status", "title", "owner"]) })).toThrow(
        new DatabaseError("INVALID_QUERY", message),
    );
});
