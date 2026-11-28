import { TerminalSize } from "@destack/environment";
import { space } from "@destack/account/object";
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, presentable } from "@destack/object";
import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";
import { Frame } from "./frame.ts";

/** A card titled by its headline, with fields that sort before it. */
const card = defineObject({
    name: "card",
    plural: "cards",
    title: t`${plural(argument("count"), { one: "Card", other: "Cards" })}`,
    scope: space,
    fields: {
        /** The card's headline. */
        headline: field.string(),
        /** The card's author. */
        author: field.string(),
        /** The card's body. */
        body: field.string(),
    },
    traits: [presentable({ title: "headline" })],
});

test("list a type's title first, then its other fields, its identifier last", () => {
    expect(Frame.columns(card, { kind: "table" })).toEqual(["headline", "author", "body", "id"]);
});

/** Issues moving from todo through started to done, with an estimate and a due time. */
const issue = defineObject({
    name: "issue",
    plural: "issues",
    title: t`${plural(argument("count"), { one: "Issue", other: "Issues" })}`,
    scope: space,
    fields: {
        /** The issue's title. */
        title: field.string(),
        /** Where the issue stands. */
        state: field.state({
            initial: "todo",
            final: ["done"],
            transitions: {
                start: { from: ["todo"], to: "started", permission: "write" },
                finish: { from: ["started"], to: "done", permission: "write" },
            },
        }),
        /** The points the issue takes. */
        estimate: field.integer().optional(),
        /** When the issue is due. */
        dueAt: field
            .time()
            .labeled(t`Due at`)
            .optional(),
    },
    traits: [presentable({ title: "title" })],
});

/** The moment the tables draw at, 2026-11-19 12:00 UTC. */
const NOW = Date.UTC(2026, 10, 19, 12);

/** Two cards as the space returns them. */
const cards = [
    { id: "card-1", headline: "Launch", author: "ada", body: "Write the post", color: null },
    { id: "card-2", headline: "Intel start", author: "grace", body: "Fix it", color: "red" },
];

test("show a person a table with the identifiers last and the rows counted", () => {
    expect(
        new Frame(TerminalFixture.terminal(TerminalSize.VT100), NOW, new Map()).table(
            cards,
            Frame.columns(card, { kind: "table" }),
            card,
        ),
    ).toBe(
        [
            "  headline       author  body            id",
            "  ─────────────  ──────  ──────────────  ──────",
            "  Launch         ada     Write the post  card-1",
            "  ● Intel start  grace   Fix it          card-2",
            "",
            "  2 cards",
        ].join("\n"),
    );
});

test("show a person one object under its title and identifier with its shown fields, and a program every field", () => {
    const [launch] = cards;
    expect({
        person: new Frame(TerminalFixture.terminal(TerminalSize.VT100), NOW, new Map()).record(
            launch ?? {},
            card,
        ),
        program: new Frame(TerminalFixture.terminal(), NOW, new Map()).record(launch ?? {}, card),
    }).toEqual({
        person: ["  Launch  card-1", "", "  author  ada", "  body    Write the post"].join("\n"),
        program: "id\tcard-1\nheadline\tLaunch\nauthor\tada\nbody\tWrite the post\ncolor\t-",
    });
});

test("draw states by stage, numbers at the end and near times relative, fitted to the terminal", () => {
    // draw three issues at 80 columns, and every field at 40, where columns hide
    const issues = [
        {
            id: "issue-1",
            title: "Write the release notes for the beta and send them to every early user",
            state: "todo",
            estimate: 3,
            dueAt: NOW + 2 * 86_400_000,
            color: null,
        },
        {
            id: "issue-2",
            title: "Ship the beta",
            state: "started",
            estimate: 13,
            dueAt: Date.UTC(2026, 11, 24),
            color: null,
        },
        {
            id: "issue-3",
            title: "Tag v1",
            state: "done",
            estimate: null,
            dueAt: NOW - 3 * 3_600_000,
            color: null,
        },
    ];
    const every = ["title", "state", "estimate", "dueAt", "color", "id"];
    const narrow = TerminalFixture.terminal({ columns: 40, rows: 24 });

    expect({
        wide: new Frame(TerminalFixture.terminal(TerminalSize.VT100), NOW, new Map())
            .table(issues, Frame.columns(issue, { kind: "table" }), issue)
            .split("\n"),
        narrow: new Frame(narrow, NOW, new Map()).table(issues, every, issue).split("\n"),
    }).toEqual({
        wide: [
            "  title                                state      estimate  Due at       id",
            "  ───────────────────────────────────  ─────────  ────────  ───────────  ───────",
            "  Write the release notes for the be…  ○ todo            3  in 2 days    issue-1",
            "  Ship the beta                        ◐ started        13  Thu, Dec 24  issue-2",
            "  Tag v1                               ● done     –         3 hours ago  issue-3",
            "",
            "  3 issues",
        ],
        narrow: [
            "  title     state     id       …",
            "  ────────  ────────  ───────  ─",
            "  Write t…  ○ todo    issue-1  …",
            "  Ship th…  ◐ start…  issue-2  …",
            "  Tag v1    ● done    issue-3  …",
            "",
            "  3 issues",
        ],
    });
});

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

test("draw steps as a tree under their parents with their key fields, and depths for a program", () => {
    const tree = { kind: "tree", id: "span", parent: "parent" } as const;

    expect({
        person: new Frame(TerminalFixture.terminal(TerminalSize.VT100), NOW, new Map()).draw(
            steps,
            tree,
            step,
        ),
        program: new Frame(TerminalFixture.terminal(), NOW, new Map()).draw(steps, tree, step),
    }).toEqual({
        person: [
            "  request       done     s1",
            "  ├─ read       done     s2",
            "  └─ write      running  s3",
            "     └─ commit  queued   s4",
            "",
            "  4 steps",
        ].join("\n"),
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
        new Frame(TerminalFixture.terminal(TerminalSize.VT100), NOW, new Map())
            .draw(steps, timeline, step)
            .split("\n"),
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

test("draw steps on a board by state as cards side by side, and as records for a program", () => {
    const board = { kind: "board", by: "state", empty: "show" } as const;

    expect({
        person: new Frame(TerminalFixture.terminal(TerminalSize.VT100), NOW, new Map()).draw(
            steps,
            board,
            step,
        ),
        program: new Frame(TerminalFixture.terminal(), NOW, new Map()).draw(steps, board, step),
    }).toEqual({
        person: [
            "  queued 1                  running 1                 done 2",
            "  commit                    write                     request",
            "  d · c                     c · a                     a · –",
            "                                                      read",
            "                                                      b · a",
        ].join("\n"),
        program: ["queued\tcommit", "running\twrite", "done\trequest", "done\tread"].join("\n"),
    });
});

test("draw issues on a calendar: the month beside its agenda when it fits, else above it", () => {
    // place three issues: one due three hours ago, two due in two days
    const rows = [
        {
            id: "issue-1",
            title: "Write the release notes",
            state: "todo",
            estimate: 3,
            dueAt: NOW + 2 * 86_400_000,
            color: null,
        },
        {
            id: "issue-2",
            title: "Ship the beta",
            state: "started",
            estimate: 13,
            dueAt: NOW + 2 * 86_400_000,
            color: null,
        },
        {
            id: "issue-3",
            title: "Tag v1",
            state: "done",
            estimate: null,
            dueAt: NOW - 3 * 3_600_000,
            color: null,
        },
    ];
    const calendar = { kind: "calendar", by: "dueAt" } as const;
    const window = { start: Date.UTC(2026, 10, 1), end: Date.UTC(2026, 11, 1) };
    const draw = (size: TerminalSize) =>
        new Frame(TerminalFixture.terminal(size), NOW, new Map())
            .draw(rows, calendar, issue, window)
            .split("\n");

    expect({ wide: draw(TerminalSize.VT100), narrow: draw({ columns: 40, rows: 24 }) }).toEqual({
        wide: [
            "  November 2026         │ Thu, Nov 19",
            "  Su Mo Tu We Th Fr Sa  │   Tag v1                   ● done     –   issue-3",
            "   1  2  3  4  5  6  7  │ Sat, Nov 21",
            "   8  9 10 11 12 13 14  │   Write the release notes  ○ todo      3  issue-1",
            "  15 16 17 18 19 20 21  │   Ship the beta            ◐ started  13  issue-2",
            "  22 23 24 25 26 27 28  │",
            "  29 30                 │",
            "",
            "  3 issues",
        ],
        narrow: [
            "  November 2026",
            "  Su Mo Tu We Th Fr Sa",
            "   1  2  3  4  5  6  7",
            "   8  9 10 11 12 13 14",
            "  15 16 17 18 19 20 21",
            "  22 23 24 25 26 27 28",
            "  29 30",
            "",
            "  Thu, Nov 19",
            "    Tag v1        ● done     –   issue-3",
            "  Sat, Nov 21",
            "    Write the r…  ○ todo      3  issue-1",
            "    Ship the be…  ◐ started  13  issue-2",
            "",
            "  3 issues",
        ],
    });
});
