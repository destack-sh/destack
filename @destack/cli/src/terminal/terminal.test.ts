import { TerminalSize } from "@destack/environment";
import { testHost } from "@destack/test/host";
import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";
import { Terminal } from "./terminal.ts";

/** Open a terminal over a process, a person's or a program's, with colour or without. */
function open(isTerminal: boolean, variables: Readonly<Record<string, string>> = {}): Terminal {
    const input = isTerminal ? { terminal: TerminalSize.VT100 } : {};
    const running = TerminalFixture.process(variables, testHost().scheduler, input);

    return new Terminal(running, { timeZone: "UTC", locale: "en-US" });
}

/** Write an event of each state. */
function events(terminal: Terminal): string[] {
    return [
        terminal.event("Building", "running", "it"),
        terminal.event("Built", "done", "it"),
        terminal.event("error", "failed", "it"),
        terminal.event("Storing", "step", "it"),
    ];
}

test("write an event, a question and a hint with their word in one gutter, and a heading with its identifier", () => {
    // write each kind of line for a person, without colour
    const person = open(true, { NO_COLOR: "1" });
    expect([
        person.event("Building", "running", "package '@ada/notes'", "3 outputs"),
        person.event("Published", "done", "package '@ada/notes' 2026.11.1", "in 2.1s"),
        person.event("error", "failed", "task 'task-01zzz' not found"),
        person.question("Approve this plan?"),
        person.hint("next", "destack install @ada/notes"),
        person.heading("Write the launch post", "task-01a2f"),
    ]).toEqual([
        "  Building  package '@ada/notes' 3 outputs",
        " Published  package '@ada/notes' 2026.11.1 in 2.1s",
        "     error  task 'task-01zzz' not found",
        "         ?  Approve this plan? yes/no",
        "      next  destack install @ada/notes",
        "Write the launch post  task-01a2f",
    ]);
});

test("paint an event's word by its state for a person, and write a program the words alone", () => {
    // paint the word orange while it runs, green once done, red once failed, on a truecolor terminal
    const person = open(true, { COLORTERM: "truecolor" });
    const program = open(false);
    expect({ person: events(person), program: events(program) }).toEqual({
        person: [
            "\x1b[38;2;255;121;46m  Building\x1b[39m  it",
            "\x1b[38;2;48;164;108m     Built\x1b[39m  it",
            "\x1b[38;2;229;72;77m     error\x1b[39m  it",
            "   Storing  it",
        ],
        program: ["Building  it", "Built  it", "error  it", "Storing  it"],
    });
});

test("paint a swatch in 24-bit colour on a truecolor terminal, else in the nearest of 256", () => {
    expect(
        [open(true, { COLORTERM: "truecolor" }), open(true)].map((terminal) =>
            terminal.swatch("red", "x"),
        ),
    ).toEqual(["\x1b[38;2;229;72;77mx\x1b[39m", "\x1b[38;5;167mx\x1b[39m"]);
});
