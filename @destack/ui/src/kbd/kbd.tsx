import * as style from "@destack/style";
import { color, radius, space, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { Shortcut } from "@destack/locale";
import { type Accessor, createSignal, For, type JSX, omit, onSettled } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The styles of keys and key groups. */
const styles = style.create({
    kbd: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        gap: space[1],
        width: "fit-content",
        minWidth: `calc(1lh + ${space[1]})`,
        paddingBlock: `calc(${space[1]} / 2)`,
        paddingInline: space[1],
        borderRadius: radius[2],
        backgroundColor: color.selected,
        color: color.mutedForeground,
        fontWeight: weight.medium,
        pointerEvents: "none",
        userSelect: "none",
    },
    group: {
        display: "inline-flex",
        alignItems: "center",
        gap: space[1],
    },
});

/** The properties of a key or key group, the native element's attributes included. */
export type KbdProperties = Omit<JSX.HTMLAttributes<HTMLElement>, "class"> & ElementPartProperties;

/** Render a key of a keyboard shortcut. */
export function Kbd(properties: KbdProperties): JSX.Element {
    return renderPart("kbd", "kbd", properties, [text.xs, styles.kbd]);
}

/** Render the keys of one shortcut side by side. */
export function KbdGroup(properties: KbdProperties): JSX.Element {
    return renderPart("kbd", "kbd-group", properties, styles.group);
}

/** Read whether the viewer's platform labels `mod` as Command, false until the page settles. */
export function useCommandPlatform(): Accessor<boolean> {
    // read the user agent once mounted, so the server and the first render agree
    const [isCommand, setCommand] = createSignal(false, { ownedWrite: true });
    onSettled(() => {
        setCommand(Shortcut.isCommand(navigator.userAgent));
    });

    return isCommand;
}

/** The properties of a key combination, the native element's attributes included. */
export type KbdShortcutProperties = KbdProperties & {
    /** The key combination in the keybinding syntax, such as `mod+k`. */
    readonly keys: string;
};

/** Render a key combination such as `mod+k`, a key each, as the viewer's platform labels them. */
export function KbdShortcut(properties: KbdShortcutProperties): JSX.Element {
    // label the keys for the viewer's platform
    const isCommand = useCommandPlatform();
    const rest = omit(properties, "keys");

    return (
        <KbdGroup data-slot="kbd-shortcut" {...rest}>
            <For each={Shortcut.labels(properties.keys, isCommand())}>
                {(label) => <Kbd>{label}</Kbd>}
            </For>
        </KbdGroup>
    );
}
