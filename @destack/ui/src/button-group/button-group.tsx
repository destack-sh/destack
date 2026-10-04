import * as style from "@destack/style";
import { color, radius, shadow, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import { merge, omit } from "solid-js";
import { JoinContext, useJoin } from "../join/index.ts";
import { Separator, type SeparatorProperties } from "../separator/index.ts";

/** The orientation of a button group that sets none. */
const DEFAULTS: Required<Pick<ButtonGroupProperties, "orientation">> = {
    orientation: "horizontal",
};

/** The styles of a button group and its elements. */
const styles = style.create({
    group: {
        display: "flex",
        alignItems: "stretch",
        width: "fit-content",
        gap: { default: 0, ":has(> [data-slot=button-group])": space[2] },
    },
    text: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        paddingInline: space[4],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.muted,
        boxShadow: shadow.raised,
        fontWeight: weight.medium,
    },
    separator: {
        position: "relative",
        alignSelf: "stretch",
        margin: 0,
        borderColor: color.input,
    },
});

/** The direction of each orientation. */
const orientations = style.create({
    horizontal: {
        flexDirection: "row",
    },
    vertical: {
        flexDirection: "column",
    },
});

/** The direction a button group lines its buttons up in. */
export type ButtonGroupOrientation = "horizontal" | "vertical";

/** The properties of an element of a button group, the native element's attributes included. */
export type ButtonGroupElementProperties = Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style"
> & {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
};

/** The properties of a button group, the native element's attributes included. */
export interface ButtonGroupProperties extends ButtonGroupElementProperties {
    /** The direction, horizontal by default. */
    readonly orientation?: ButtonGroupOrientation;
}

/** Render buttons, inputs and selects joined into one control, or groups of them spaced apart. */
export function ButtonGroup(properties: ButtonGroupProperties): JSX.Element {
    const group = merge(DEFAULTS, properties);
    const rest = omit(group, "orientation", "style");

    return (
        <JoinContext value={() => group.orientation}>
            <div
                data-slot="button-group"
                data-orientation={group.orientation}
                role="group"
                {...rest}
                {...style.attrs(styles.group, orientations[group.orientation], group.style)}
            />
        </JoinContext>
    );
}

/** Render a label or icon joined to a group's buttons. */
export function ButtonGroupText(properties: ButtonGroupElementProperties): JSX.Element {
    const join = useJoin();
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="button-group-text"
            {...rest}
            {...style.attrs(text.callout, styles.text, join(), properties.style)}
        />
    );
}

/** Render a line between a group's buttons, vertical by default. */
export function ButtonGroupSeparator(properties: SeparatorProperties): JSX.Element {
    const separator = merge({ orientation: "vertical" } as const, properties);

    return (
        <Separator
            data-slot="button-group-separator"
            {...separator}
            style={[styles.separator, separator.style]}
        />
    );
}
