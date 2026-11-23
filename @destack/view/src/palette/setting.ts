import { DeclarationKey } from "@destack/package/declare";
import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";
import { PaletteSource } from "./entry.ts";
import { Shortcut, t } from "@destack/locale";

/** The key combination that opens the command palette from anywhere. */
export const shortcut = defineSetting({
    name: "palette.shortcut",
    title: t`Command palette shortcut`,
    description: t`The key combination opening the command palette from anywhere.`,
    schema: Shortcut,
    default: "alt+space",
    scope: "user",
    overrides: ["device"],
    apply: "restart",
});

/** The key combinations that run actions, by action, merged key by key from every scope. */
export const keybindings = defineSetting({
    name: "keybindings",
    title: t`Keybindings`,
    description: t`The key combinations running actions, by action; null unbinds an action.`,
    schema: schema.record(DeclarationKey, Shortcut.nullable()),
    default: {},
    scope: "user",
    overrides: ["device"],
    merge: "key",
});

/** The sources the command palette offers, by source, merged key by key from every scope. */
export const sources = defineSetting({
    name: "palette.sources",
    title: t`Command palette sources`,
    description: t`Whether the command palette offers each source: windows, spaces, recent objects, and each package's commands and views by package; false turns one off.`,
    schema: schema.partialRecord(PaletteSource, schema.boolean()),
    default: {},
    scope: "user",
    overrides: ["device"],
    merge: "key",
});
