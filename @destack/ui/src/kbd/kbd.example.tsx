import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Stack } from "../stack/index.ts";
import { Kbd, KbdGroup, KbdShortcut } from "./kbd.tsx";

/** The shortcut that opens the command menu. */
export const kbdCommandShortcut = defineExample({
    of: Kbd,
    name: "commandShortcut",
    description: t`The shortcut that opens the command menu.`,
    render: () => (
        <p>
            Open the command menu with{" "}
            <KbdGroup>
                <Kbd>⌘</Kbd>
                <Kbd>K</Kbd>
            </KbdGroup>
        </p>
    ),
});

/** The shortcuts of a note's actions, one per row, labelled for the viewer's platform. */
export const kbdNoteShortcuts = defineExample({
    of: KbdShortcut,
    name: "noteShortcuts",
    description: t`The shortcuts of a note's actions, one per row, labelled for the viewer's platform.`,
    render: () => (
        <Stack space="2">
            <p>
                Save <KbdShortcut keys="mod+s" />
            </p>
            <p>
                Find <KbdShortcut keys="mod+f" />
            </p>
            <p>
                Close <KbdShortcut keys="escape" />
            </p>
        </Stack>
    ),
});
