import { DeclarationReference } from "@destack/package/declare";
import { Shortcut } from "@destack/locale";
import { schema } from "@destack/schema";
import { type ActionDescription, ActionFocus } from "@destack/object";

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
    /** Bind an action to the person's key combination for it, else its declared one, if any. */
    of(
        action: DeclarationReference,
        declared: { readonly key: Shortcut | undefined; readonly focus: ActionFocus },
        overrides: Readonly<Record<string, Shortcut | null>>,
    ): Keybinding | undefined {
        const settingKey = DeclarationReference.key(action);
        const key = settingKey in overrides ? overrides[settingKey] : declared.key;

        return key === null || key === undefined
            ? undefined
            : { action, key, focus: declared.focus };
    },
});
/** A key combination running an action. */
export type Keybinding = schema.Infer<typeof keybinding>;

/** An action calling a method that a package declares, as its build describes it. */
export interface ActionDeclaration extends DeclarationReference {
    /** The action's description. */
    readonly description: ActionDescription;
}
