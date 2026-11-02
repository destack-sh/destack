import { onCleanup, useContext } from "solid-js";
import type { ShortcutText } from "@destack/object";
import type { JsonObject, schema } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import { ViewMountContext } from "./context.ts";
import type { CommandRegistration } from "../palette/registry.ts";

/** Track the objects the person selects in the running view while the component is mounted. */
export function registerSelection(read: () => readonly ObjectReference[]): void {
    const unregister = useContext(ViewMountContext).commands.select(read);
    onCleanup(unregister);
}

/** Register a command of the running view while the calling component is mounted. */
export function registerCommand<
    Input extends schema.Schema<JsonObject>,
    const Keys extends string = never,
>(command: CommandRegistration<Input> & { readonly keybinding?: Keys & ShortcutText<Keys> }): void {
    const unregister = useContext(ViewMountContext).commands.register(command);
    onCleanup(unregister);
}
