import { through } from "@destack/access";
import { TerminalSize } from "@destack/environment";
import { space } from "@destack/account/object";
import { argument, type KeyPress, plural, t } from "@destack/locale";
import { defineObject, field, presentable } from "@destack/object";
import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";
import { Frame } from "./frame.ts";
import { Pager, type PagerAnswer, type PagerStep } from "./pager.ts";

/** Tickets moving from todo through started to done, titled, edited and deleted. */
const ticket = defineObject(
    {
        name: "ticket",
        plural: "tickets",
        title: t`${plural(argument("count"), { one: "Ticket", other: "Tickets" })}`,
        scope: space,
        fields: {
            /** The ticket's title. */
            title: field.string(),
            /** Where the ticket stands. */
            state: field.state({
                initial: "todo",
                final: ["done"],
                transitions: {
                    start: { from: ["todo"], to: "started", permission: "write" },
                    finish: { from: ["started"], to: "done", permission: "write" },
                },
            }),
        },
        access: {
            permissions: { read: through("scope", "read"), write: through("scope", "read") },
        },
        traits: [presentable({ title: "title" })],
    },
    (method) => ({
        edit: method.update("write", { fields: ["title"] }),
        delete: method.delete("write"),
    }),
);

/** Two tickets: one to do, one started. */
const ROWS = [
    { id: "ticket-1", title: "Write the notes", state: "todo", color: null },
    { id: "ticket-2", title: "Ship the beta", state: "started", color: null },
];

/** The moment the pager draws at. */
const NOW = Date.UTC(2026, 10, 19, 12);

/** Press a sequence of keys, a name such as Enter or a character each, returning the last step. */
function pressed(keys: readonly string[]): PagerAnswer {
    // draw the tickets as a table, and press each key in order
    const frame = new Frame(TerminalFixture.terminal(TerminalSize.VT100), NOW, new Map());
    const page = frame.page(ROWS, { kind: "table" }, ticket);
    let answered: PagerAnswer = {
        pager: Pager.start(ticket, []),
        step: { kind: "draw" },
    };
    for (const key of keys) {
        const event: KeyPress = {
            key,
            code: key,
            altKey: false,
            ctrlKey: false,
            metaKey: false,
            shiftKey: false,
        };
        answered = answered.pager.press(event, page, ticket.view, frame, TerminalSize.VT100.rows);
    }

    return answered;
}

/** Read a step as the method its action calls, its targets and its input, else its kind. */
function ran(step: PagerStep): unknown {
    return step.kind === "run"
        ? {
              method: step.action.call?.method,
              targets: step.targets.map((row) => row["id"]),
              input: step.input,
          }
        : step.kind;
}

test("move a ticket through its next transition, ask an edit's fields, and confirm a delete", () => {
    expect({
        start: ran(pressed(["s", "Enter"]).step),
        finish: ran(pressed(["ArrowDown", "s", "Enter"]).step),
        edit: ran(pressed(["e", "N", "e", "w", "Enter"]).step),
        refused: ran(pressed([":", "d", "e", "l", "Enter", "n", "o", "Enter"]).step),
        deleted: ran(pressed([":", "d", "e", "l", "Enter", "y", "e", "s", "Enter"]).step),
    }).toEqual({
        start: { method: "start", targets: ["ticket-1"], input: {} },
        finish: { method: "finish", targets: ["ticket-2"], input: {} },
        edit: { method: "edit", targets: ["ticket-1"], input: { title: "New" } },
        refused: "draw",
        deleted: { method: "delete", targets: ["ticket-1"], input: {} },
    });
});

test("run an action on every selected ticket, filter by title, switch views, and quit", () => {
    expect({
        both: ran(
            pressed([" ", "ArrowDown", " ", ":", "d", "e", "l", "Enter", "y", "e", "s", "Enter"])
                .step,
        ),
        filter: pressed(["/", "s", "h", "Enter"]).pager.filter,
        view: pressed(["v"]).step,
        quit: ran(pressed(["q"]).step),
    }).toEqual({
        both: { method: "delete", targets: ["ticket-1", "ticket-2"], input: {} },
        filter: "sh",
        view: { kind: "view", view: ticket.views[1] },
        quit: "quit",
    });
});
