import * as style from "@destack/style";
import { color, radius, size, space, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, omit } from "@destack/view";

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
export interface KbdProperties extends Omit<JSX.HTMLAttributes<HTMLElement>, "class"> {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
}

/** Render a key of a keyboard shortcut. */
export function Kbd(properties: KbdProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <kbd
            data-slot="kbd"
            {...rest}
            {...style.attributes([text.caption, styles.kbd, properties.xstyle], properties.style)}
        />
    );
}

/** Render the keys of one shortcut side by side. */
export function KbdGroup(properties: KbdProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <kbd
            data-slot="kbd-group"
            {...rest}
            {...style.attributes([styles.group, properties.xstyle], properties.style)}
        />
    );
}
