import { PackageId } from "@destack/package";
import { expect, test } from "@destack/test";
import { type JsonObject, schema } from "@destack/schema";
import { flush } from "../solid/reactive.ts";
import { draw, wait } from "../test/dom.ts";
import type { CommandEntry, PaletteClient, PaletteEntry } from "../palette/entry.ts";
import { CommandPalette } from "./palette.tsx";

/** The package declaring the tasks. */
const TASKS = PackageId.parse("package-019f5530-8000-7000-8000-0000000000a1");

/** The person's work space. */
const WORK = schema.identifier("space").parse("space-019f5530-8000-7000-8000-0000000000b1");

/** The person's home. */
const HOME = schema.identifier("space").parse("space-019f5530-8000-7000-8000-0000000000b2");

/** Completing a task, which acts on one task. */
const COMPLETE: CommandEntry = {
    kind: "command",
    space: WORK,
    installation: "work",
    packageId: TASKS,
    name: "complete",
    title: "Complete task",
    typePackageId: TASKS,
    type: "task",
    method: "complete",
    keybinding: "mod+enter",
};

/** Snoozing a task, which acts on one task and asks until when. */
const SNOOZE: CommandEntry = {
    ...COMPLETE,
    name: "snooze",
    title: "Snooze task",
    method: "snooze",
    keybinding: null,
};

/** The palette's entries: two commands and a view of the work space, a window, both spaces and a note opened lately at home. */
const ENTRIES: readonly PaletteEntry[] = [
    COMPLETE,
    SNOOZE,
    {
        kind: "view",
        space: WORK,
        installation: "work",
        packageId: TASKS,
        name: "board",
        title: "Board",
    },
    {
        kind: "window",
        id: "window-1",
        title: "Inbox",
        link: "destack://home.personal.florian/home",
    },
    { kind: "space", id: WORK, title: "work.acme" },
    { kind: "space", id: HOME, title: "personal.florian" },
    {
        kind: "object",
        space: HOME,
        object: { packageId: TASKS, type: "task", scope: HOME, id: "task-7" },
        title: "Water the plants",
    },
];

/** A client answering from memory, recording the calls and openings, with one space it could not read. */
function client(calls: unknown[]): PaletteClient {
    return {
        list: async () => ({ entries: ENTRIES, failures: ["cannot read garden.acme"] }),
        shape: async (command) => ({
            isTargeted: true,
            input:
                command.name === "snooze"
                    ? {
                          type: "object",
                          properties: { until: { type: "string" } },
                          required: ["until"],
                      }
                    : { type: "object", properties: {} },
        }),
        objects: async (_command, title) =>
            [
                { id: "task-1", title: "Write the intro" },
                { id: "task-2", title: "Review the outro" },
            ].filter((object) => object.title.toLowerCase().includes(title.toLowerCase())),
        call: async (command, input: JsonObject) => {
            calls.push([command.name, input]);

            return {};
        },
        open: async (entry) => {
            calls.push(["open", entry.title]);
        },
        focus: async (window) => {
            calls.push(["focus", window.id]);
        },
        close: () => calls.push(["close"]),
    };
}

/** List the text of a palette's options. */
function options(container: HTMLElement): (string | null)[] {
    return [...container.querySelectorAll("[role=option]")].map((option) => option.textContent);
}

/** Press Escape inside a palette. */
function escape(container: HTMLElement): void {
    container
        .querySelector("[data-slot=command-palette]")
        ?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    flush();
}

/** Choose the option whose text starts with a text. */
function choose(container: HTMLElement, text: string): void {
    const option = [...container.querySelectorAll<HTMLElement>("[role=option]")].find((element) =>
        element.textContent?.startsWith(text),
    );
    option?.click();
    flush();
}

test("run a command on the focused window's object, and on a picked one with its input", async () => {
    const calls: unknown[] = [];
    const focus = {
        object: { packageId: TASKS, type: "task", scope: "space-1", id: "task-9" },
    };

    // complete the focused task right away
    const first = draw(() => <CommandPalette client={client(calls)} focus={focus} />);
    await wait(0);
    flush();
    const offered = options(first);
    const failures = first.querySelector("[data-slot=command-failures]")?.textContent;
    choose(first, "Complete task");
    await wait(0);
    flush();
    const outcome = first.querySelector("[data-slot=command-outcome]")?.getAttribute("data-state");

    // snooze a picked task until a time typed into its form
    const second = draw(() => <CommandPalette client={client(calls)} focus={{}} />);
    await wait(0);
    flush();
    choose(second, "Snooze task");
    await wait(0);
    flush();
    choose(second, "Review the outro");
    const until = second.querySelector<HTMLInputElement>("input[name=until]");
    if (until !== null) {
        until.value = "tomorrow";
    }
    second
        .querySelector("form")
        ?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    await wait(0);
    flush();

    // open the board and the plants, and focus the inbox's window
    const third = draw(() => <CommandPalette client={client(calls)} focus={{}} />);
    await wait(0);
    flush();
    choose(third, "Board");
    choose(third, "Water the plants");
    choose(third, "Inbox");
    await wait(0);

    expect({ offered, failures, outcome, calls }).toEqual({
        offered: [
            "Complete taskwork.acmemod+enter",
            "Snooze taskwork.acme",
            "Inbox",
            "Boardwork.acme",
            "Water the plantspersonal.florian",
            "personal.florian",
            "work.acme",
        ],
        failures: "cannot read garden.acme",
        outcome: "done",
        calls: [
            ["complete", { id: "task-9" }],
            ["snooze", { until: "tomorrow", id: "task-2" }],
            ["open", "Board"],
            ["open", "Water the plants"],
            ["focus", "window-1"],
            ["close"],
        ],
    });
});

test("narrow the search to a chosen space, widening it on Escape before closing", async () => {
    const calls: unknown[] = [];
    const container = draw(() => <CommandPalette client={client(calls)} focus={{}} />);
    await wait(0);
    flush();

    // choose the home, then press Escape twice
    choose(container, "personal.florian");
    const narrowed = [
        container.querySelector("[data-slot=command-scope]")?.textContent,
        options(container),
    ];
    escape(container);
    const widened = options(container).length;
    escape(container);

    expect({ narrowed, widened, calls }).toEqual({
        narrowed: ["personal.florian", ["Water the plantspersonal.florian", "personal.florian"]],
        widened: 7,
        calls: [["close"]],
    });
});

test("run a command the shell launched only once the person chooses it in the palette", async () => {
    const calls: unknown[] = [];
    const focus = {
        object: { packageId: TASKS, type: "task", scope: WORK, id: "task-9" },
    };
    const container = draw(() => (
        <CommandPalette
            client={client(calls)}
            focus={focus}
            command={{ packageId: TASKS, name: "complete" }}
        />
    ));
    await wait(0);
    flush();

    // read the offered command before choosing it, then choose it
    const offered = [options(container), [...calls]];
    choose(container, "Complete task");
    await wait(0);

    expect({ offered, calls }).toEqual({
        offered: [["Complete task"], []],
        calls: [["complete", { id: "task-9" }]],
    });
});
