import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";
import { CommandReference } from "../declare/command.ts";
import { PaletteSource } from "./entry.ts";
import { Accelerator } from "./accelerator.ts";

/** The key combination that opens the command palette from anywhere. */
export const shortcut = defineSetting({
    name: "palette.shortcut",
    title: "Command palette shortcut",
    description: "The key combination opening the command palette from anywhere.",
    schema: Accelerator,
    default: "alt+space",
    scope: "user",
    overrides: ["device"],
    apply: "restart",
});

/** The key combinations that run commands, by command, merged key by key from every scope. */
export const keybindings = defineSetting({
    name: "keybindings",
    title: "Keybindings",
    description: "The key combinations running commands, by command; null unbinds a command.",
    schema: schema.record(CommandReference.key, Accelerator.nullable()),
    default: {},
    scope: "user",
    overrides: ["device"],
    apply: "immediate",
    merge: "key",
});

/** The sources the command palette offers, by source, merged key by key from every scope. */
export const sources = defineSetting({
    name: "palette.sources",
    title: "Command palette sources",
    description:
        "Whether the command palette offers each source: windows, spaces, recent objects, and each package's commands and views by package; false turns one off.",
    schema: schema.partialRecord(PaletteSource, schema.boolean()),
    default: {},
    scope: "user",
    overrides: ["device"],
    apply: "immediate",
    merge: "key",
});
