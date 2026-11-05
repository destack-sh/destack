import { testSimulation } from "@destack/object/test";
import { expect, test } from "@destack/test";
import { desktopObjects } from "../test/index.ts";
import { window } from "./window.ts";

test("open a view's window, count each request into focus, two at one instant too, and place it", async () => {
    const objects = await desktopObjects(testSimulation().environment("desktop"));
    const windows = objects.mutate(window);

    // open a view's window, ask it into focus twice without the clock moving, and place it
    const link = "destack://notes--personal--florian/home";
    const opened = await windows.open({ link }).predicted;
    await windows.focus({ id: opened.id }).confirmed;
    await windows.focus({ id: opened.id }).confirmed;
    const bounds = { x: 40, y: 60, width: 900, height: 700 };
    await windows.place({ id: opened.id, bounds }).confirmed;
    const placed = await objects.query.window.findFirst({ where: { id: opened.id } });

    // expect a view with no requests or place at first, then both requests counted and the place stored
    expect({
        opened: [opened.link, opened.kind, opened.focuses, opened.bounds],
        placed: [placed?.link, placed?.kind, placed?.focuses, placed?.bounds],
    }).toEqual({
        opened: [link, "view", 0, null],
        placed: [link, "view", 2, bounds],
    });
});
