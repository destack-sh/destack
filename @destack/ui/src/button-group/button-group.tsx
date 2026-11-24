import type { Axis } from "../position/index.ts";
import * as style from "@destack/style";
import { color, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, merge, omit, untrack } from "@destack/view";
import { DivideContext, JoinContext, useJoin } from "../join/index.ts";
import { Separator, type SeparatorProperties } from "../separator/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

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
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.muted,
        fontWeight: weight.medium,
    },
    separator: {
        position: "relative",
        alignSelf: "stretch",
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

/** The properties of an element of a button group, the native element's attributes included. */
export type ButtonGroupElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** The properties of a button group, the native element's attributes included. */
export interface ButtonGroupProperties extends ButtonGroupElementProperties {
    /** The direction, horizontal by default. */
    readonly orientation?: Axis;
    /** Whether a line sets filled controls apart, false by default. */
    readonly divided?: boolean;
}

/** Render buttons, inputs and selects joined into one control, or groups of them spaced apart. */
export function ButtonGroup(properties: ButtonGroupProperties): JSX.Element {
    const group = merge(DEFAULTS, properties);
    const rest = omit(group, "orientation", "divided", "xstyle", "style");

    return (
        <JoinContext value={() => group.orientation}>
            <DivideContext value={untrack(() => group.divided === true)}>
                <div
                    data-slot="button-group"
                    data-orientation={group.orientation}
                    role="group"
                    {...rest}
                    {...style.attributes(
                        [styles.group, orientations[group.orientation], group.xstyle],
                        group.style,
                    )}
                />
            </DivideContext>
        </JoinContext>
    );
}

/** Render a label or icon joined to a group's buttons. */
export function ButtonGroupText(properties: ButtonGroupElementProperties): JSX.Element {
    const join = useJoin();

    return renderPart("div", "button-group-text", properties, () => [text.sm, styles.text, join()]);
}

/** Render a line between a group's buttons, vertical by default. */
export function ButtonGroupSeparator(properties: SeparatorProperties): JSX.Element {
    const separator = merge({ orientation: "vertical" } as const, properties);

    return (
        <Separator
            data-slot="button-group-separator"
            {...separator}
            xstyle={[styles.separator, separator.xstyle]}
        />
    );
}
