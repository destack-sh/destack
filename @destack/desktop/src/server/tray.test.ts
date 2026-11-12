import { Catalog, Localization, SOURCE_LOCALE, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import type { MenuEntry } from "./menu.ts";
import { Tray } from "./tray.ts";

/** The language the tray shows in: the one its messages are written in. */
const SOURCE = Localization.of(SOURCE_LOCALE, []);

/** The space the machine serves shown by its name. */
const NOTES = schema.identifier("space").parse("space-019f5530-8000-7000-8000-000000000001");

/** The space the machine serves shown by its title. */
const PERSONAL = schema.identifier("space").parse("space-019f5530-8000-7000-8000-000000000002");

/** The machine a space moves to. */
const CLOUD = schema.identifier("machine").parse("machine-019f5530-8000-7000-8000-000000000003");

/** The space shown by its title. */
const PERSONAL_SPACE = { name: "personal", title: "Personal" };

/** The space shown by its name. */
const NOTES_SPACE = { name: "notes", title: null };

/** The machine as it serves: enabled. */
const SERVING = { status: "enabled" as const };

/** The placements of both spaces, neither moving. */
const PLACED = [
    { spaceId: PERSONAL, targetId: null, space: PERSONAL_SPACE },
    { spaceId: NOTES, targetId: null, space: NOTES_SPACE },
];

/** Read a section's entries as the person sees them: each title, marked when it runs nothing. */
function titles(entries: readonly MenuEntry[]): string[] {
    return entries.map((entry) => (entry.kind === "label" ? `(${entry.title})` : entry.title));
}

test("show the spaces this computer serves by title below the status line, each opening its space", async () => {
    // plan the tray, then choose each space
    const opened: string[] = [];
    const view = Tray.view(
        { machine: SERVING, tunnel: "open", placements: PLACED },
        async (space) => {
            opened.push(space);
        },
        SOURCE,
    );
    for (const entry of view.entries) {
        if (entry.kind === "action") {
            await entry.run();
        }
    }

    // expect the status line, the spaces sorted by title, and each opening its space
    expect({ ...view, entries: titles(view.entries), opened }).toEqual({
        status: "serving",
        tooltip: "Destack: ⁨Serving 2 spaces⁩",
        entries: ["(Serving 2 spaces)", "notes", "Personal"],
        opened: [NOTES, PERSONAL],
    });
});

/** Open no space, for views whose spaces nobody chooses. */
async function openNothing(): Promise<void> {}

test("read syncing, paused, offline and withdrawn machines in that order of precedence", () => {
    // move one space away, drain the machine, lose its tunnel, and withdraw it
    const moving = [
        { spaceId: PERSONAL, targetId: CLOUD, space: PERSONAL_SPACE },
        { spaceId: NOTES, targetId: null, space: NOTES_SPACE },
    ];
    const views = [
        Tray.view({ machine: SERVING, tunnel: "open", placements: moving }, openNothing, SOURCE),
        Tray.view(
            { machine: { ...SERVING, status: "draining" }, tunnel: "open", placements: moving },
            openNothing,
            SOURCE,
        ),
        Tray.view({ machine: SERVING, tunnel: "dialing", placements: moving }, openNothing, SOURCE),
        Tray.view({ machine: undefined, tunnel: "open", placements: PLACED }, openNothing, SOURCE),
        Tray.view({ machine: undefined, tunnel: "open", placements: [] }, openNothing, SOURCE),
    ];

    // expect each status line and the moving space marked
    expect(views.map(({ status, tooltip, entries }) => [status, tooltip, titles(entries)])).toEqual(
        [
            [
                "syncing",
                "Destack: ⁨Syncing 2 spaces⁩",
                ["(Syncing 2 spaces)", "notes", "⁨Personal⁩ (Syncing)"],
            ],
            ["paused", "Destack: ⁨Paused⁩", ["(Paused)", "notes", "⁨Personal⁩ (Syncing)"]],
            ["offline", "Destack: ⁨Offline⁩", ["(Offline)", "notes", "⁨Personal⁩ (Syncing)"]],
            [
                "error",
                "Destack: ⁨This computer left its account⁩",
                ["(This computer left its account)", "notes", "Personal"],
            ],
            [
                "error",
                "Destack: ⁨This computer left its account⁩",
                ["(This computer left its account)"],
            ],
        ],
    );
});

test("show the tray in the host's language, translating its status line where a catalog does", () => {
    // translate the paused line into German
    const paused = t`Paused`;
    const catalog = Catalog.parse({
        package: paused.package,
        locale: "de",
        messages: { [paused.id]: "Pausiert" },
    });
    const german = Localization.of("de", [catalog]);

    // plan a drained machine's tray in German
    const view = Tray.view(
        { machine: { ...SERVING, status: "draining" }, tunnel: "open", placements: PLACED },
        openNothing,
        german,
    );

    // expect the German line, in the tooltip too
    expect({ line: view.entries[0]?.title, tooltip: view.tooltip }).toEqual({
        line: "Pausiert",
        tooltip: "Destack: ⁨Pausiert⁩",
    });
});
