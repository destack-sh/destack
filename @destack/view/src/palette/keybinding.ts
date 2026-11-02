import { Shortcut, CommandReference } from "@destack/object";
import type { schema } from "@destack/schema";

/** The user agents of platforms whose `mod` key is Command. */
const COMMAND_PLATFORM = /Mac|iPhone|iPad/u;

/** A key combination bound to a command, by the command's settings key. */
export interface Keybinding {
    /** The command's key, as `<package>/<name>`. */
    readonly command: string;
    /** The key combination running it. */
    readonly keybinding: Shortcut;
}

/** A command a package declares, with the key combination it declares. */
export const CommandDeclaration = CommandReference.extend({
    /** The key combination the package declares, absent for none. */
    keybinding: Shortcut.exactOptional(),
});
/** A command a package declares, with its key combination. */
export type CommandDeclaration = schema.Infer<typeof CommandDeclaration>;

/** Report whether the browser runs on a platform whose `mod` key is Command. */
export function isCommandPlatform(): boolean {
    return COMMAND_PLATFORM.test(navigator.userAgent);
}
