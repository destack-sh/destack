import type { ObjectClient } from "@destack/object/client";
import { testSimulation } from "@destack/object/test";
import { expect, test } from "@destack/test";
import { Link } from "../link/index.ts";
import { type DESKTOP_OBJECTS, window, type WindowRow } from "../object/index.ts";
import { desktopObjects, recordShell } from "../test/index.ts";
import { present } from "@destack/schema";
import { Dialog } from "./dialog.ts";
import { Window, type WindowState } from "./window.ts";

/** The links of the stable desktop. */
const links = new Link("destack");

/** The view the windows open. */
const NOTES = "destack://notes--personal--florian/home";

/** The board the windows open. */
const TASKS = "destack://tasks--personal--florian/board";

/** Take a window the shell shows for a row, over the objects storing the row. */
function opened(
    row: WindowRow,
    url: string,
    objects: ObjectClient<typeof DESKTOP_OBJECTS>,
): Window {
    const address = present(links.parse(row.link), `the address of ${row.link}`);
    const context = {
        objects,
        shell: recordShell().shell,
        report: (error: unknown) => {
            throw error;
        },
    };

    return new Window({ ...row, url, address }, context);
}

/** Take the state of a window the shell shows for a row. */
function shown(
    row: WindowRow,
    url: string,
    objects: ObjectClient<typeof DESKTOP_OBJECTS>,
): WindowState {
    return { kind: "shown", window: opened(row, url, objects) };
}

test("plan shown windows from window rows: show new rows, merge duplicate links, focus on request and remove closed rows", async () => {
    const simulation = testSimulation();
    const objects = await desktopObjects(simulation.environment("desktop"));
    const windows = objects.mutate(window);

    // open notes twice and the board once a millisecond apart, so they order as opened
    const notes = await windows.open({ link: NOTES }).predicted;
    await simulation.jump(simulation.clock.now() + 1);
    const again = await windows.open({ link: NOTES }).predicted;
    await simulation.jump(simulation.clock.now() + 1);
    const board = await windows.open({ link: TASKS }).predicted;
    const first = await objects.query.window.findMany();

    // show notes, then ask for its focus and close the board
    const native = new Map([
        [notes.id, shown(notes, "http://notes--personal--florian.localhost:7470/home", objects)],
    ]);
    await windows.focus({ id: notes.id }).confirmed;
    await windows.close({ id: board.id }).confirmed;
    native.set(
        board.id,
        shown(board, "http://tasks--personal--florian.localhost:7470/board", objects),
    );
    const second = await objects.query.window.findMany();

    // expect the duplicate merged into the earliest row and the closed board removed
    expect({
        first: Window.plan(new Map(), first),
        second: Window.plan(native, second),
    }).toEqual({
        first: [
            { kind: "show", row: first.find((row) => row.id === notes.id) },
            { kind: "merge", id: again.id, into: notes.id },
            { kind: "show", row: first.find((row) => row.id === board.id) },
        ],
        second: [
            { kind: "focus", row: second.find((row) => row.id === notes.id) },
            { kind: "merge", id: again.id, into: notes.id },
            { kind: "remove", id: board.id, isClosing: true },
        ],
    });
});

test("plan a palette apart from a view window opening the same link, showing both", async () => {
    // open a view and a palette at the same link
    const simulation = testSimulation();
    const objects = await desktopObjects(simulation.environment("desktop"));
    const windows = objects.mutate(window);
    const link = "destack://home--personal--florian/command";
    const view = await windows.open({ link }).predicted;
    await simulation.jump(simulation.clock.now() + 1);
    const palette = await windows.open({ link, kind: "palette" }).predicted;
    const rows = await objects.query.window.findMany();

    // show each of them, merging neither into the other
    expect(Window.plan(new Map(), rows)).toEqual([
        { kind: "show", row: rows.find((row) => row.id === view.id) },
        { kind: "show", row: rows.find((row) => row.id === palette.id) },
    ]);
});

test("plan nothing for a window launching or asking, and remove one asking once its row closes", async () => {
    // open notes and the board, one launching and one asking the person
    const simulation = testSimulation();
    const objects = await desktopObjects(simulation.environment("desktop"));
    const windows = objects.mutate(window);
    const notes = await windows.open({ link: NOTES }).predicted;
    await simulation.jump(simulation.clock.now() + 1);
    const board = await windows.open({ link: TASKS }).predicted;
    const address = present(links.parse(TASKS), "the board's address");
    const states = new Map<string, WindowState>([
        [notes.id, { kind: "launching" }],
        [board.id, { kind: "asking", dialog: new Dialog(board.id, address) }],
    ]);
    const open = await objects.query.window.findMany();

    // close the board while its dialog asks
    await windows.close({ id: board.id }).confirmed;
    const closed = await objects.query.window.findMany();

    // expect no step while both wait, then the asking board removed
    expect({ open: Window.plan(states, open), closed: Window.plan(states, closed) }).toEqual({
        open: [],
        closed: [{ kind: "remove", id: board.id, isClosing: true }],
    });
});

test("write only the last place of a drag once the window rests, and none once the window stops", async () => {
    const simulation = testSimulation();
    const objects = await desktopObjects(simulation.environment("desktop"));
    const windows = objects.mutate(window);
    const notes = await windows.open({ link: NOTES }).predicted;
    const board = await windows.open({ link: TASKS }).predicted;
    const notesWindow = opened(
        notes,
        "http://notes--personal--florian.localhost:7470/home",
        objects,
    );
    const boardWindow = opened(
        board,
        "http://tasks--personal--florian.localhost:7470/board",
        objects,
    );

    // drag notes through three places, and move the board before it stops
    const last = { x: 30, y: 30, width: 800, height: 600 };
    notesWindow.place({ x: 10, y: 10, width: 800, height: 600 });
    notesWindow.place({ x: 20, y: 20, width: 800, height: 600 });
    notesWindow.place(last);
    boardWindow.place({ x: 50, y: 50, width: 640, height: 480 });
    boardWindow.stop();

    // read the places before and after the windows rest
    const before = await objects.query.window.findMany();
    await simulation.run(simulation.clock.now() + 1_000);
    const after = await objects.query.window.findMany();

    // expect nothing written while dragging, then notes' last place in one write after its open
    expect({
        before: before.map((row) => [row.link, row.bounds]),
        after: after.map((row) => [row.link, row.bounds, row.generation]),
    }).toEqual({
        before: [
            [NOTES, null],
            [TASKS, null],
        ],
        after: [
            [NOTES, last, 2],
            [TASKS, null, 1],
        ],
    });
});
