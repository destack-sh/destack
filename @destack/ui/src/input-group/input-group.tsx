import * as style from "@destack/style";
import {
    color,
    motion,
    radius,
    shadow,
    size,
    space,
    stroke,
    weight,
} from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import { merge, omit } from "solid-js";
import { Button, type ButtonProperties } from "../button/index.ts";
import { Input, type InputProperties } from "../input/index.ts";
import { JoinContext, useJoin } from "../join/index.ts";
import { Textarea, type TextareaProperties } from "../textarea/index.ts";

/** The alignment of an addon that sets none. */
const ADDON_DEFAULTS: Required<Pick<InputGroupAddonProperties, "align">> = {
    align: "inline-start",
};

/** The variant, size and type of a group's button that sets none. */
const BUTTON_DEFAULTS: Required<Pick<InputGroupButtonProperties, "variant" | "size" | "type">> = {
    variant: "ghost",
    size: "xs",
    type: "button",
};

/** A group whose control is focused. */
const FOCUSED = ":has(> [data-slot=input-group-control]:focus-visible)";

/** A group whose control is invalid. */
const INVALID = ":has(> [data-slot=input-group-control][aria-invalid=true])";

/** A group holding a textarea. */
const MULTILINE = ":has(> textarea[data-slot=input-group-control])";

/** A group stacking an addon above its control. */
const STACKED_START = ":has(> [data-slot=input-group-addon][data-align=block-start])";

/** A group stacking an addon below its control. */
const STACKED_END = ":has(> [data-slot=input-group-addon][data-align=block-end])";

/** The styles of an input group and its elements. */
const styles = style.create({
    group: {
        position: "relative",
        display: "flex",
        flexDirection: { default: "row", [STACKED_START]: "column", [STACKED_END]: "column" },
        alignItems: { default: "center", [STACKED_START]: "stretch", [STACKED_END]: "stretch" },
        width: "100%",
        minWidth: 0,
        height: {
            default: size[3],
            [MULTILINE]: "auto",
            [STACKED_START]: "auto",
            [STACKED_END]: "auto",
        },
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: {
            default: color.input,
            [FOCUSED]: color.ring,
            [INVALID]: color.destructive,
        },
        borderRadius: radius[3],
        boxShadow: shadow.inset,
        transitionProperty: "border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: {
            default: "none",
            [FOCUSED]: "solid",
        },
        outlineWidth: stroke.ring,
        outlineColor: {
            default: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
            [INVALID]: `color-mix(in oklab, ${color.destructive} 20%, transparent)`,
        },
    },
    control: {
        flex: 1,
        borderWidth: 0,
        borderRadius: 0,
        backgroundColor: "transparent",
        boxShadow: "none",
        outlineStyle: "none",
    },
    textarea: {
        resize: "none",
        paddingBlock: space[3],
    },
    addon: {
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        gap: space[2],
        paddingBlock: space[1],
        color: color.mutedForeground,
        fontWeight: weight.medium,
        cursor: "text",
        userSelect: "none",
    },
    button: {
        boxShadow: "none",
    },
    text: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        color: color.mutedForeground,
    },
});

/** The place and padding of an addon at each alignment. */
const alignments = style.create({
    "inline-start": { order: -1, paddingInlineStart: space[3] },
    "inline-end": { order: 1, paddingInlineEnd: space[3] },
    "block-start": {
        order: -1,
        justifyContent: "flex-start",
        width: "100%",
        paddingInline: space[3],
        paddingTop: space[3],
    },
    "block-end": {
        order: 1,
        justifyContent: "flex-start",
        width: "100%",
        paddingInline: space[3],
        paddingBottom: space[3],
    },
});

/** Where an addon sits around its group's control: before or after it in the line, or above or below it. */
export type InputGroupAlign = "inline-start" | "inline-end" | "block-start" | "block-end";

/** The properties of an element of an input group, the native element's attributes included. */
export type InputGroupElementProperties<Attributes = JSX.HTMLAttributes<HTMLDivElement>> = Omit<
    Attributes,
    "class" | "style"
> & {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
};

/** The properties of an input group's addon, the native element's attributes included. */
export interface InputGroupAddonProperties extends InputGroupElementProperties {
    /** Where the addon sits, inline-start by default. */
    readonly align?: InputGroupAlign;
}

/** The properties of an input group's button, a ghost extra-small button by default. */
export type InputGroupButtonProperties = ButtonProperties;

/** Render an input or textarea framed together with its icons, text and buttons. */
export function InputGroup(properties: InputGroupElementProperties): JSX.Element {
    // join the group to its neighbours, never its own control and buttons
    const join = useJoin();
    const rest = omit(properties, "style", "children");

    return (
        <div
            data-slot="input-group"
            role="group"
            {...rest}
            {...style.attrs(styles.group, join(), properties.style)}
        >
            <JoinContext value={() => undefined}>{properties.children}</JoinContext>
        </div>
    );
}

/** Render icons, text or buttons beside or around a group's control, focusing the control on a click outside a button. */
export function InputGroupAddon(properties: InputGroupAddonProperties): JSX.Element {
    const addon = merge(ADDON_DEFAULTS, properties);
    const rest = omit(addon, "align", "style", "onClick");

    return (
        <div
            data-slot="input-group-addon"
            data-align={addon.align}
            role="group"
            {...rest}
            onClick={(event) => {
                focusControl(event);
                if (typeof addon.onClick === "function") {
                    addon.onClick(event);
                }
            }}
            {...style.attrs(text.callout, styles.addon, alignments[addon.align], addon.style)}
        />
    );
}

/** Focus the control of an addon's group unless the click pressed a button. */
function focusControl(event: MouseEvent & { readonly currentTarget: HTMLDivElement }): void {
    if (event.target instanceof Element && event.target.closest("button") !== null) {
        return;
    }
    event.currentTarget.parentElement
        ?.querySelector<HTMLElement>("[data-slot=input-group-control]")
        ?.focus();
}

/** Render a button inside a group, ghost and extra small unless set otherwise. */
export function InputGroupButton(properties: InputGroupButtonProperties): JSX.Element {
    const button = merge(BUTTON_DEFAULTS, properties);

    return <Button {...button} style={[styles.button, button.style]} />;
}

/** Render text or an icon inside an addon. */
export function InputGroupText(
    properties: InputGroupElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return <span {...rest} {...style.attrs(text.callout, styles.text, properties.style)} />;
}

/** Render a group's input, tied to its field like any input. */
export function InputGroupInput(properties: InputProperties): JSX.Element {
    return (
        <Input
            data-slot="input-group-control"
            {...properties}
            style={[styles.control, properties.style]}
        />
    );
}

/** Render a group's textarea, tied to its field like any textarea. */
export function InputGroupTextarea(properties: TextareaProperties): JSX.Element {
    return (
        <Textarea
            data-slot="input-group-control"
            {...properties}
            style={[styles.control, styles.textarea, properties.style]}
        />
    );
}
