import * as style from "@destack/style";
import { color, radius, size, space, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The styles of keys and key groups. */
const styles = style.create({
    kbd: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        gap: space[1],
        width: "fit-content",
        minWidth: size[1],
        height: size[1],
        paddingInline: space[1],
        borderRadius: radius[2],
        backgroundColor: color.muted,
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
    return renderPart("kbd", "kbd", properties, [text.caption, styles.kbd]);
}

/** Render the keys of one shortcut side by side. */
export function KbdGroup(properties: KbdProperties): JSX.Element {
    return renderPart("kbd", "kbd-group", properties, styles.group);
}
