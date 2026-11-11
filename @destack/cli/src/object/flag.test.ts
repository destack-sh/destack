import { space } from "@destack/account/object";
import { expect, test, refusal } from "@destack/test";
import { principal, relation, union } from "@destack/access";
import { Filter } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

import { FlagSet } from "./flag.ts";

/** A task with a person assigned, a due time, a priority and a state. */
const task = defineObject(
    {
        name: "task",
        plural: "tasks",
        scope: space,
        fields: {
            title: field.string(schema.string().min(1)),
            assignee: field.reference(principal.user).optional(),
            due: field.time().optional(),
            priority: field.enum(["low", "normal", "high"]).default("normal"),
            status: field.state({
                initial: "open",
                transitions: { complete: { from: ["open"], to: "done", permission: "edit" } },
            }),
        },
        permissions: { read: union(relation("assignee")), edit: relation("assignee") },
    },
    (method) => ({
        create: method.create("edit"),
    }),
);

/** Tuesday 6 October 2026, 15:00 local time, when the flags are read. */
const NOW = Date.UTC(2026, 9, 6, 15, 0);

/** Read a local calendar day as its midnight. */
function day(month: number, date: number): number {
    return Date.UTC(2026, month - 1, date);
}

test("restrict a list by flags of its row fields: any of comma values, the person, dates and weekdays", () => {
    const flags = FlagSet.rows(task);
    const filter = flags.filter("user-1", NOW, "UTC");

    // offer enumerations as choices and read references by their relation
    const options = flags.options();
    expect([options["priority"], options["assignee"]]).toEqual([
        {
            type: "string",
            description: "The priority: low, normal, high",
            argument: "low|normal|high",
        },
        { type: "string", description: "The assignee", argument: "id" },
    ]);

    // read each flag's values as any of them, the person for @me and local dates for time fields
    expect(
        flags.condition(
            { status: "open,done", assignee: "@me", due: "2026-10-09", title: "Ship it" },
            filter,
        ),
    ).toEqual({
        AND: [
            { title: "Ship it" },
            { assigneeId: "user-1" },
            { due: day(10, 9) },
            { OR: [{ status: "open" }, { status: "done" }] },
        ],
    });

    // read a filter's weekdays as the coming ones, today included
    expect(Filter.parse("due < @friday OR due = @tuesday OR due > @monday", filter)).toEqual({
        OR: [{ due: { lt: day(10, 9) } }, { due: day(10, 6) }, { due: { gt: day(10, 12) } }],
    });
});

test("build a method's input from flags over a JSON input, refusing values outside an enumeration", async () => {
    const flags = FlagSet.input(task, "create");
    const filter = flags.filter("user-1", NOW, "UTC");

    // write the flags over the JSON input, leaving the scope and request to the call
    expect(Object.keys(flags.options())).toEqual(["title", "assignee", "due", "priority"]);
    expect(
        flags.input(
            { title: "Ship it", assignee: "@me", due: "@tomorrow", priority: "high" },
            { title: "Draft", status: "open" },
            filter,
        ),
    ).toEqual({
        title: "Ship it",
        status: "open",
        assigneeId: "user-1",
        due: day(10, 7),
        priority: "high",
    });
    expect(await refusal(() => flags.input({ priority: "urgent" }, {}, filter))).toEqual([
        "BAD_REQUEST",
        "USAGE_INVALID",
        "--'priority' is invalid: it takes one of 'low, normal, high'",
    ]);
});

test("read today and the coming days in the person's time zone, across a change of its offset", () => {
    // stand at 23:30 UTC on 24 October 2026, already the 25th in Vienna, whose offset falls from +2 to +1 that night
    const filter = FlagSet.rows(task).filter(
        undefined,
        Date.UTC(2026, 9, 24, 23, 30),
        "Europe/Vienna",
    );

    // start each day at Vienna's midnight: +2 before the change, +1 after it
    expect([
        filter.values?.["today"],
        filter.values?.["tomorrow"],
        filter.values?.["sunday"],
    ]).toEqual([
        Date.UTC(2026, 9, 24, 22, 0),
        Date.UTC(2026, 9, 25, 23, 0),
        Date.UTC(2026, 9, 24, 22, 0),
    ]);
});
