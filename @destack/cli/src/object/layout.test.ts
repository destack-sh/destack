import { space } from "@destack/account/object";
import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";
import { LayoutRender } from "./layout.ts";

/** Steps of a call, nested under their parents, timed by a start and a duration in microseconds. */
const step = defineObject({
    name: "step",
    plural: "steps",
    title: t`${plural(argument("count"), { one: "Step", other: "Steps" })}`,
    scope: space,
    fields: {
        name: field.string(),
        span: field.string(),
        parent: field.string().optional(),
        state: field.enum(["queued", "running", "done"]),
        time: field.integer(),
        duration: field.integer(),
    },
    presentable: {
        title: "name",
        icon: false,
        color: false,
        views: [
            {
                name: "tree",
                title: t`Tree`,
                layout: { kind: "tree", id: "span", parent: "parent" },
            },
            {
                name: "waterfall",
                title: t`Waterfall`,
                layout: {
                    kind: "timeline",
                    start: "time",
                    end: { duration: "duration" },
                    unit: "microsecond",
                },
            },
        ],
    },
});

/** A call's steps: the request, its read and its write, the write still running. */
const steps = [
    {
        id: "s1",
        name: "request",
        span: "a",
        parent: null,
        state: "done",
        time: 0,
        duration: 40_000,
    },
    { id: "s2", name: "read", span: "b", parent: "a", state: "done", time: 2000, duration: 8000 },
    {
        id: "s3",
        name: "write",
        span: "c",
        parent: "a",
        state: "running",
        time: 12_000,
        duration: 20_000,
    },
    {
        id: "s4",
        name: "commit",
        span: "d",
        parent: "c",
        state: "queued",
        time: 30_000,
        duration: 1000,
    },
];

test("draw steps as a tree under their parents, with guides for a person and depths for a program", () => {
    const tree = { kind: "tree", id: "span", parent: "parent" } as const;

    expect({
        person: LayoutRender.render(steps, tree, step, TerminalFixture.terminal(true)),
        program: LayoutRender.render(steps, tree, step, TerminalFixture.terminal(false)),
    }).toEqual({
        person: ["  request  a", "  ├─ read  b", "  └─ write  c", "     └─ commit  d"].join("\n"),
        program: ["0\ta\trequest", "1\tb\tread", "1\tc\twrite", "2\td\tcommit"].join("\n"),
    });
});

test("draw steps as a waterfall: a tick axis, eighth-block bars nested under their parents, the critical path", () => {
    const timeline = {
        kind: "timeline",
        start: "time",
        end: { duration: "duration" },
        unit: "microsecond",
    } as const;

    expect(
        LayoutRender.render(steps, timeline, step, TerminalFixture.terminal(true)).split("\n"),
    ).toEqual([
        "                0             10ms          20ms          30ms       40ms",
        "                ┬─────────────┬─────────────┬─────────────┬─────────────┬",
        "  request       █████████████████████████████████████████████████████████   40ms",
        "  ├─ read         ████████████▎                                            8.0ms",
        "  └─ write                       ████████████████████████████▋              20ms",
        "     └─ commit                                            ██▏              1.0ms",
        "",
        "  4 steps over 40ms · 4 on the critical path",
    ]);
});

test("draw steps on a board by state, side by side in the enum's order, and as records for a program", () => {
    const board = { kind: "board", by: "state", empty: "show" } as const;

    expect({
        person: LayoutRender.render(steps, board, step, TerminalFixture.terminal(true)),
        program: LayoutRender.render(steps, board, step, TerminalFixture.terminal(false)),
    }).toEqual({
        person: [
            `  queued 1${" ".repeat(16)}  running 1${" ".repeat(15)}  done 2${" ".repeat(18)}`,
            `  ▪ commit${" ".repeat(16)}  ▪ write${" ".repeat(17)}  ▪ request${" ".repeat(15)}`,
            `  ${" ".repeat(24)}  ${" ".repeat(24)}  ▪ read${" ".repeat(18)}`,
        ].join("\n"),
        program: ["queued\tcommit", "running\twrite", "done\trequest", "done\tread"].join("\n"),
    });
});
