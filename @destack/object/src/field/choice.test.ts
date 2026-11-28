import { none } from "@destack/access";
import { argument, Localization, plural, SOURCE_LOCALE, t } from "@destack/locale";
import { schema, toJsonSchema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { BoardLayout } from "../layout/board.ts";
import { defineObject, ObjectDescription, ObjectTypePool } from "../object/index.ts";
import { Choice } from "../presentation/choice.ts";
import { ObjectValue } from "../presentation/value.ts";
import { orderable } from "../trait/index.ts";
import { space } from "../test/space.ts";
import { RICH_TEXT } from "../sequence/rich.ts";
import { field } from "./field.ts";

/** The locale declarations are written in. */
const SOURCE = Localization.of(SOURCE_LOCALE, []);

/** Statuses people order and add themselves, each in a category code reads. */
const status = defineObject({
    name: "status",
    plural: "statuses",
    title: t`${plural(argument("count"), { one: "Status", other: "Statuses" })}`,
    scope: space,
    traits: [orderable()],
    fields: {
        name: field.string(),
        category: field.enum(["open", "done"]),
    },
});

/** Tasks with a titled priority, a plain size, a titled state and one of the people's statuses. */
const task = defineObject({
    name: "task",
    plural: "tasks",
    title: t`${plural(argument("count"), { one: "Task", other: "Tasks" })}`,
    scope: space,
    traits: [orderable({ within: "status", move: "edit" })],
    fields: {
        title: field.string(),
        priority: field.enum({
            low: { title: t`Low`, icon: "arrow-down", color: "gray" },
            urgent: { title: t`Urgent`, icon: "warning", color: "red" },
        }),
        size: field.enum(["s", "l"]),
        review: field.state({
            initial: "asked",
            final: ["approved"],
            states: {
                asked: { title: t`Asked` },
                approved: { title: t`Approved`, color: "green" },
            },
            transitions: {
                approve: { from: ["asked"], to: "approved", permission: "edit" },
            },
        }),
        status: field.reference(status),
    },
    access: { permissions: { edit: none() } },
});

test("list an enum's and a state's values in order, each looking as declared, else as stored", () => {
    expect(
        [task.fields.priority, task.fields.size, task.fields.review, task.fields.title].map(
            (each) => each.choices?.map((choice) => Choice.of(choice, SOURCE)),
        ),
    ).toEqual([
        [
            { value: "low", title: "Low", icon: { icon: "arrow-down" }, color: "gray" },
            { value: "urgent", title: "Urgent", icon: { icon: "warning" }, color: "red" },
        ],
        [
            { value: "s", title: "s" },
            { value: "l", title: "l" },
        ],
        [
            { value: "asked", title: "Asked" },
            { value: "approved", title: "Approved", color: "green" },
        ],
        undefined,
    ]);
});

test("take an integer's choices by number, and store only the numbers chosen from", () => {
    // declare a rank of three numbers, two looking as declared
    const rank = field.integer({
        0: { title: t`None`, icon: "minus" },
        1: { title: t`Urgent`, color: "red" },
        4: { title: t`Low` },
    });
    const column = rank.column("rank", "test", "durable");

    expect({
        choices: rank.choices?.map((choice) => Choice.of(choice, SOURCE)),
        stored: [0, 1, 2, 4].map((value) => column.definition.schema.safeParse(value).success),
        // write the numbers as JSON Schema's enum, as a method's input describes them
        described: toJsonSchema(column.definition.schema),
    }).toEqual({
        choices: [
            { value: 0, title: "None", icon: { icon: "minus" } },
            { value: 1, title: "Urgent", color: "red" },
            { value: 4, title: "Low" },
        ],
        stored: [true, true, false, true],
        described: {
            $schema: "https://json-schema.org/draft/2020-12/schema",
            type: "number",
            enum: [0, 1, 4],
        },
    });
});

test("refuse integer choices keyed by no whole number", () => {
    expect(() => field.integer({ high: { title: t`High` } })).toThrow(
        new TypeError("an integer field's choices are keyed by whole numbers"),
    );
});

test("describe how an enum's and a state's values look, and declare them the same from data", () => {
    // declare the type anew from its description as JSON stores it
    const stored = schema.json().parse(JSON.parse(JSON.stringify(ObjectDescription.of(task))));
    const [declared] = ObjectTypePool.of([space, status]).declare([
        ObjectDescription.parse(stored),
    ]);

    expect(["priority", "size", "review"].map((name) => declared?.fields[name]?.choices)).toEqual([
        task.fields.priority.choices,
        task.fields.size.choices,
        task.fields.review.choices,
    ]);
});

test("refuse a look of a state no transition reaches, and an enum of no value", () => {
    // declare a state machine with a state too many, and an empty enum
    expect(() =>
        field.state({
            initial: "asked",
            final: ["approved"],
            states: { lost: { title: t`Lost` } },
            transitions: { approve: { from: ["asked"], to: "approved", permission: "edit" } },
        }),
    ).toThrow(new TypeError("state 'lost' is invalid: no transition reaches it"));
    expect(() => field.enum({})).toThrow(new TypeError("an enum field needs a value"));
});

test("read a stored value with its title, one no longer listed as stored", () => {
    expect([
        ObjectValue.of(task, "priority", "urgent", SOURCE),
        ObjectValue.of(task, "priority", "gone", SOURCE),
        ObjectValue.of(task, "review", "approved", SOURCE),
    ]).toEqual([
        {
            kind: "choice",
            value: "urgent",
            title: "Urgent",
            icon: { icon: "warning" },
            color: "red",
        },
        { kind: "choice", value: "gone", title: "gone" },
        { kind: "state", value: "approved", title: "Approved", color: "green", stage: "final" },
    ]);
});

test("offer a board by each enum, state and reference to ordered objects, its columns the objects", () => {
    // group two tasks by the people's statuses, one with a status that is gone
    const layouts = task.layouts.filter((each) => each.kind === "board");
    const layout = { kind: "board", by: "statusId", empty: "show" } as const;
    const statuses: Choice[] = [
        { value: "status-1", title: "Triage" },
        { value: "status-2", title: "Shipped" },
    ];
    const rows = [
        { title: "a", statusId: "status-2" },
        { title: "b", statusId: "status-9" },
    ];
    const groups = BoardLayout.group(layout, rows, statuses, SOURCE);

    expect({
        layouts: layouts.map((each) => each.by),
        listed: BoardLayout.choices(layout, task.fields, SOURCE),
        groups: groups.map((group) => [group.choice, group.rows.map((row) => row["title"])]),
    }).toEqual({
        layouts: ["priority", "size", "review", "statusId"],
        listed: undefined,
        groups: [
            [{ value: null, title: "No value" }, ["b"]],
            [{ value: "status-1", title: "Triage" }, []],
            [{ value: "status-2", title: "Shipped" }, ["a"]],
        ],
    });
});

test("order objects within a reference's objects, a move writing the reference", () => {
    expect({ ordering: task.ordering, moves: task.method("move").fields }).toEqual({
        ordering: { within: ["statusId"] },
        moves: ["statusId"],
    });
});

test("keep a text field's marks and nodes through its modifiers, as a label", () => {
    expect([
        field.text(RICH_TEXT).labeled(t`Notes`).textSchema,
        field.string().labeled(t`Title`).textSchema,
    ]).toEqual([field.text(RICH_TEXT).textSchema, undefined]);
});
