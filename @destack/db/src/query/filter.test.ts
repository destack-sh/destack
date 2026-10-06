import { expect, test } from "@destack/test";
import { DatabaseError } from "../error/error.ts";
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

test("nest dotted names under relations, or name one field when a filter reads attribute keys", () => {
    // walk the relation, or keep the dotted key whole
    expect([
        Filter.parse("project.status = open"),
        Filter.parse("session.status = crashed", { isRelational: false }),
    ]).toEqual([{ project: { status: "open" } }, { "session.status": "crashed" }]);
});

test("decide a filter over plain rows as SQL decides it, dotted keys named whole", () => {
    // match crashed sessions of one release, and anything but them
    const crashed = Filter.compile('session.status = crashed AND release = "2026.10.1"');
    const healthy = Filter.compile("NOT session.status = crashed");
    const rows = [
        { "session.status": "crashed", release: "2026.10.1" },
        { "session.status": "crashed", release: "2026.10.0" },
        { "session.status": "ok", release: "2026.10.1" },
    ];
    expect([rows.map(crashed), rows.map(healthy)]).toEqual([
        [true, false, false],
        [false, false, true],
    ]);
});

test("refuse a filter on a field outside the filterable ones", () => {
    // refuse a field the caller may not filter by
    expect(() =>
        Filter.parse("secret = x", { fields: new Set(["status", "title", "owner"]) }),
    ).toThrow(new DatabaseError("INVALID_QUERY", "unknown field secret at 0 in filter secret = x"));
});
