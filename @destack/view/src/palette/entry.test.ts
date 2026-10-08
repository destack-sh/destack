import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Accelerator, type KeyPress } from "./accelerator.ts";
import { CommandReference } from "../declare/command.ts";
import { Palette, type PaletteEntry } from "./entry.ts";

/** The package declaring the tasks and their commands. */
const TASKS = PackageId.parse("package-019f5530-8000-7000-8000-0000000000a1");

/** The package declaring the notes. */
const NOTES = PackageId.parse("package-019f5530-8000-7000-8000-0000000000a2");

/** The person's work space, with tasks and notes. */
const WORK = schema.identifier("space").parse("space-019f5530-8000-7000-8000-0000000000b1");

/** The person's home, with notes. */
const HOME = schema.identifier("space").parse("space-019f5530-8000-7000-8000-0000000000b2");

/** A command of an installation calling a method of one of its types. */
function command(
    space: typeof WORK,
    installation: string,
    packageId: PackageId,
    name: string,
    title: string,
    type: string,
): PaletteEntry {
    return {
        kind: "command",
        space,
        installation,
        packageId,
        name,
        title,
        typePackageId: packageId,
        type,
        method: name,
        keybinding: null,
    };
}

/** Press a key with some modifiers held. */
function press(
    key: string,
    modifiers: Partial<Record<"altKey" | "ctrlKey" | "metaKey" | "shiftKey", boolean>>,
    code = `Key${key.toUpperCase()}`,
): KeyPress {
    return {
        key,
        code,
        altKey: false,
        ctrlKey: false,
        metaKey: false,
        shiftKey: false,
        ...modifiers,
    };
}

/** The entries of a work space with tasks and notes, and of a home with notes. */
const ENTRIES: readonly PaletteEntry[] = [
    command(WORK, "work", TASKS, "complete", "Complete task", "task"),
    command(WORK, "work", TASKS, "archive", "Archive project", "project"),
    command(WORK, "notes", NOTES, "archive", "Archive note", "note"),
    command(HOME, "notes", NOTES, "archive", "Archive journal note", "note"),
    {
        kind: "view",
        space: WORK,
        installation: "notes",
        packageId: NOTES,
        name: "home",
        title: "Notes",
    },
];

test("rank entries by how their title matches, the focused window's space, installation and object first", () => {
    const titles = (query: string, focus: Parameters<typeof Palette.rank>[2]) =>
        Palette.rank(ENTRIES, query, focus).map((entry) => entry.title);
    const note = { packageId: NOTES, type: "note", scope: HOME, id: "note-1" };

    expect([
        titles("", {}),
        titles("arch", {}),
        titles("arch", { space: WORK, installation: "notes" }),
        titles("arch", { space: HOME, installation: "notes" }),
        titles("", { object: note }),
        titles("note", {}),
        titles("zzz", {}),
    ]).toEqual([
        ["Archive journal note", "Archive note", "Archive project", "Complete task", "Notes"],
        ["Archive journal note", "Archive note", "Archive project"],
        ["Archive note", "Archive project", "Archive journal note"],
        ["Archive journal note", "Archive note", "Archive project"],
        ["Archive journal note", "Archive note", "Archive project", "Complete task", "Notes"],
        ["Notes", "Archive journal note", "Archive note"],
        [],
    ]);
});

test("offer the entries of every source the person leaves on: built-in ones by name, packages by identifier", () => {
    const entries: readonly PaletteEntry[] = [
        ...ENTRIES,
        {
            kind: "window",
            id: "window-1",
            title: "Inbox",
            link: "destack://home--personal--ada/home",
        },
        { kind: "space", id: HOME, title: "personal.ada" },
        {
            kind: "object",
            space: HOME,
            object: { packageId: NOTES, type: "note", scope: HOME, id: "note-1" },
            title: "Journal",
        },
    ];
    const offered = (sources: Readonly<Record<string, boolean>>) =>
        entries.filter((entry) => Palette.isOffered(entry, sources)).map((entry) => entry.title);

    expect([
        entries.map((entry) => [Palette.source(entry), Palette.space(entry)]),
        offered({}),
        offered({ [TASKS]: false, windows: false, recent: false, spaces: true }),
    ]).toEqual([
        [
            [TASKS, WORK],
            [TASKS, WORK],
            [NOTES, WORK],
            [NOTES, HOME],
            [NOTES, WORK],
            ["windows", undefined],
            ["spaces", HOME],
            ["recent", HOME],
        ],
        [
            "Complete task",
            "Archive project",
            "Archive note",
            "Archive journal note",
            "Notes",
            "Inbox",
            "personal.ada",
            "Journal",
        ],
        ["Archive note", "Archive journal note", "Notes", "personal.ada"],
    ]);
});

test("read a command's keybinding: the person's own, none once unbound, else the declared one", () => {
    const complete = { packageId: TASKS, name: "complete" };
    const archive = { packageId: TASKS, name: "archive" };
    const overrides = {
        [CommandReference.format(complete)]: "mod+shift+c",
        [CommandReference.format(archive)]: null,
    };

    expect([
        Palette.keybinding(complete, "mod+k", overrides),
        Palette.keybinding(archive, "mod+a", overrides),
        Palette.keybinding({ packageId: NOTES, name: "archive" }, "mod+e", overrides),
        Palette.keybinding({ packageId: NOTES, name: "pin" }, undefined, overrides),
    ]).toEqual(["mod+shift+c", null, "mod+e", null]);
});

test("match key presses against accelerators, mod being Command on Command platforms and Control elsewhere, and Option letters by their key", () => {
    expect([
        Accelerator.matches("mod+k", press("k", { metaKey: true }), true),
        Accelerator.matches("mod+k", press("k", { ctrlKey: true }), true),
        Accelerator.matches("mod+k", press("k", { ctrlKey: true }), false),
        Accelerator.matches("alt+space", press(" ", { altKey: true }), true),
        Accelerator.matches("mod+shift+k", press("K", { metaKey: true, shiftKey: true }), true),
        Accelerator.matches("mod+k", press("k", { metaKey: true, shiftKey: true }), true),
        Accelerator.matches("alt+d", press("∂", { altKey: true }, "KeyD"), true),
        Accelerator.matches("alt+q", press("a", { altKey: true }, "KeyQ"), false),
        Accelerator.safeParse("mod+shift+k").success,
        Accelerator.safeParse("hyper+k").success,
    ]).toEqual([true, false, true, true, true, false, true, false, true, false]);
});

test("write accelerators with Command platform symbols, and with words joined by plus signs elsewhere", () => {
    expect([
        Accelerator.format("mod+k", true),
        Accelerator.format("mod+k", false),
        Accelerator.format("alt+shift+d", true),
        Accelerator.format("alt+shift+d", false),
        Accelerator.format("mod+enter", true),
        Accelerator.format("up", false),
    ]).toEqual(["⌘K", "Ctrl+K", "⌥⇧D", "Alt+Shift+D", "⌘Enter", "↑"]);
});
