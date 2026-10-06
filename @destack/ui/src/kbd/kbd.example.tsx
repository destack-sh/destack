import { defineExample } from "@destack/package/declare";
import { Kbd, KbdGroup } from "./kbd.tsx";

/** The shortcut that opens the command menu. */
export const kbdCommandShortcut = defineExample({
    of: Kbd,
    name: "command-shortcut",
    description: "the shortcut that opens the command menu",
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
