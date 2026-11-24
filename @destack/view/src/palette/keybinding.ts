import { DeclarationReference } from "@destack/package/declare";
import { Shortcut } from "@destack/locale";
import { schema } from "@destack/schema";
import { type ActionDescription, ActionFocus, ActionKeybinding } from "@destack/object";

/** A key combination running an action, outside form fields or anywhere. */
const keybinding = schema.object({
    /** The action it runs. */
    action: DeclarationReference,
    /** The key combination. */
    key: Shortcut,
    /** Where focus may be for the keys to run it. */
    focus: ActionFocus,
});

/** A key combination running an action, outside form fields or anywhere. */
export const Keybinding = Object.assign(keybinding, {
    /** Bind an action to the person's key combination for it, else each of its declared ones. */
    of(
        action: DeclarationReference,
        declared: {
            readonly keybinding: ActionKeybinding | undefined;
            readonly focus: ActionFocus;
        },
        overrides: Readonly<Record<string, Shortcut | null>>,
        isCommand: boolean,
    ): Keybinding[] {
        const { keybinding: keys, focus } = declared;
        const chosen = overrides[DeclarationReference.key(action)];

        return ActionKeybinding.on(keys, isCommand, chosen).map((key) => ({ action, key, focus }));
    },
});
/** A key combination running an action. */
export type Keybinding = schema.Infer<typeof keybinding>;

/** An action calling a method that a package declares, as its build describes it. */
export interface ActionDeclaration extends DeclarationReference {
    /** The action's description. */
    readonly description: ActionDescription;
}
