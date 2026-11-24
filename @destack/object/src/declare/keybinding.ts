import { Shortcut, type ShortcutText } from "@destack/locale";
import { schema } from "@destack/schema";

/** A literal key combination the compiler checks, any for one known at runtime alone. */
type KeyText<Keys extends string> = string extends Keys ? Shortcut : Keys & ShortcutText<Keys>;

/** One key combination or several, the first shown. */
type KeyTexts<Keys extends string> = KeyText<Keys> | readonly KeyText<Keys>[];

/** The key combinations of an action as its package writes them, the same on Macs unless given. */
export type ActionKeybindingDefinition<Keys extends string = Shortcut, Mac extends string = Keys> =
    | KeyTexts<Keys>
    | {
          /** The combinations on every platform but Macs. */
          readonly key: KeyTexts<Keys>;
          /** The combinations on Macs. */
          readonly mac: KeyTexts<Mac>;
      };

/** The key combinations that run an action until a person chooses others, the first shown. */
const actionKeybinding = schema.object({
    /** The combinations on every platform but Macs. */
    key: schema.array(Shortcut).min(1),
    /** The combinations on Macs. */
    mac: schema.array(Shortcut).min(1),
});

/** The key combinations that run an action until a person chooses others, the first shown. */
export const ActionKeybinding = Object.assign(actionKeybinding, {
    /** Read the combinations a package writes, on each platform. */
    of(definition: ActionKeybindingDefinition): ActionKeybinding {
        // repeat those written once on both
        const { key, mac } =
            typeof definition === "object" && "mac" in definition
                ? definition
                : { key: definition, mac: definition };

        return { key: [key].flat(), mac: [mac].flat() };
    },

    /** List the combinations on a platform: the person's, none where they unbind it, else these. */
    on(
        keybinding: ActionKeybinding | undefined,
        isCommand: boolean,
        chosen: Shortcut | null | undefined,
    ): readonly Shortcut[] {
        // take the platform's where the person chooses none
        if (chosen === undefined) {
            return (isCommand ? keybinding?.mac : keybinding?.key) ?? [];
        }

        return chosen === null ? [] : [chosen];
    },
});
/** The key combinations that run an action until a person chooses others, the first shown. */
export type ActionKeybinding = schema.Infer<typeof actionKeybinding>;
