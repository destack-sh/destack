import { t } from "@destack/locale";
import { field } from "@destack/object";
import * as style from "@destack/style";
import {
    color,
    motion,
    radius,
    size,
    space,
    stroke,
    weight,
    swatch,
} from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, merge, omit } from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { Input, type InputProperties } from "../input/index.ts";
import { JoinContext, useJoin } from "../join/index.ts";
import { Textarea, type TextareaProperties } from "../textarea/index.ts";
import { addonMarker, controlMarker, groupMarker, multilineMarker } from "./marker.stylex.ts";
import { renderPart, type Render } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The variant, size and type of a group's button that sets none. */
const BUTTON_DEFAULTS: Required<Pick<InputGroupButtonProperties, "variant" | "size" | "type">> = {
    variant: "ghost",
    size: "sm",
    type: "button",
};

/** The places an addon sits around its group's control. */
const ALIGNS = {
    "inline-start": { title: t.context("option")`Inline start` },
    "inline-end": { title: t.context("option")`Inline end` },
    "block-start": { title: t.context("option")`Block start` },
    "block-end": { title: t.context("option")`Block end` },
};

/** The properties people change in an addon's controls and inspector. */
const INPUT_GROUP_ADDON_PROPERTIES = {
    align: field
        .enum(ALIGNS)
        .labeled(t.context("label")`Align`)
        .described(t`Whether the addon sits before, after, above or below the control.`)
        .default("inline-start"),
};

/** The properties people change in a group's input's controls and inspector: its input's own. */
const INPUT_GROUP_INPUT_PROPERTIES = {
    placeholder: Input.properties.placeholder,
    disabled: Input.properties.disabled,
    readonly: Input.properties.readonly,
    required: Input.properties.required,
};

/** The styles of an input group and its elements. */
const styles = style.create({
    group: {
        position: "relative",
        display: "flex",
        flexDirection: {
            default: "row",
            [style.when.descendant('[data-align="block-start"]', addonMarker)]: "column",
            [style.when.descendant('[data-align="block-end"]', addonMarker)]: "column",
        },
        alignItems: {
            default: "center",
            [style.when.descendant('[data-align="block-start"]', addonMarker)]: "stretch",
            [style.when.descendant('[data-align="block-end"]', addonMarker)]: "stretch",
        },
        width: "100%",
        minWidth: 0,
        height: {
            default: size[2],
            [style.when.descendant('[data-slot="input-group-control"]', multilineMarker)]: "auto",
            [style.when.descendant('[data-align="block-start"]', addonMarker)]: "auto",
            [style.when.descendant('[data-align="block-end"]', addonMarker)]: "auto",
        },
        borderWidth: stroke.border,
        borderColor: {
            default: color.input,
            [style.when.descendant(":focus-visible", controlMarker)]: color.ring,
            [style.when.descendant('[aria-invalid="true"]', controlMarker)]: swatch.destructiveMark,
        },
        borderRadius: radius[3],
        opacity: {
            default: 1,
            [style.when.descendant(":disabled", controlMarker)]: 0.5,
        },
        transitionProperty: "border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: {
            default: "none",
            [style.when.descendant(":focus-visible", controlMarker)]: "solid",
        },
        outlineWidth: stroke.ring,
        outlineOffset: `calc(-1 * ${stroke.border})`,
        outlineColor: {
            default: color.ring,
            [style.when.descendant('[aria-invalid="true"]', controlMarker)]:
                `color-mix(in oklab, ${swatch.destructiveMark} 20%, transparent)`,
        },
    },
    control: {
        flex: 1,
        opacity: 1,
        // set the text in less beside an addon, which keeps its own padding
        paddingInlineStart: {
            default: space[3],
            [style.when.anySibling('[data-align="inline-start"]', addonMarker)]: space[1],
        },
        paddingInlineEnd: {
            default: space[3],
            [style.when.anySibling('[data-align="inline-end"]', addonMarker)]: space[1],
        },
    },
    textarea: {
        minHeight: space[9],
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
    "inline-start": {
        order: -1,
        // set a leading button in as far as it is from the group's top and bottom
        paddingInlineStart: {
            default: space[3],
            ":has(> [data-slot=button]:first-child)": space[1],
        },
    },
    "inline-end": {
        order: 1,
        // set a trailing button in as far as it is from the group's top and bottom
        paddingInlineEnd: { default: space[3], ":has(> [data-slot=button]:last-child)": space[1] },
    },
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

/** Where an addon sits around its group's control. */
export type InputGroupAlign = keyof typeof ALIGNS;

/** The properties of an element of an input group, the native element's attributes included. */
export type InputGroupElementProperties<Attributes = JSX.HTMLAttributes<HTMLDivElement>> = Omit<
    Attributes,
    "class"
> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
};

/** The properties of an input group's addon besides its declared ones: its native attributes. */
export type InputGroupAddonAttributes = InputGroupElementProperties;

/** The properties of an input group's button, a ghost extra-small button by default. */
export type InputGroupButtonProperties = ButtonProperties;

/** Render an input or textarea framed together with its icons, text and buttons. */
export function InputGroup(properties: InputGroupElementProperties): JSX.Element {
    // join the group to its neighbours, never its own control and buttons
    const join = useJoin();
    const rest = omit(properties, "children");

    return renderPart("div", "input-group", rest, () => [styles.group, groupMarker, join()], {
        role: "group" as const,
        get children() {
            return <JoinContext value={() => undefined}>{properties.children}</JoinContext>;
        },
    });
}

/** Render icons, text or buttons beside or around a group's control. */
export const InputGroupAddon = defineComponent({
    name: "inputGroupAddon",
    properties: INPUT_GROUP_ADDON_PROPERTIES,
    render(
        properties: RenderProperties<
            InputGroupAddonAttributes,
            typeof INPUT_GROUP_ADDON_PROPERTIES
        >,
    ): JSX.Element {
        const rest = omit(properties, "align", "onClick");

        return renderPart(
            "div",
            "input-group-addon",
            rest,
            () => [text.sm, styles.addon, alignments[properties.align], addonMarker],
            {
                get "data-align"() {
                    return properties.align;
                },
                role: "group" as const,
                onClick: (event) => {
                    focusControl(event);
                    if (typeof properties.onClick === "function") {
                        properties.onClick(event);
                    }
                },
            },
        );
    },
});

/** The properties of an input group's addon, the native element's attributes included. */
export type InputGroupAddonProperties = ComponentProperties<typeof InputGroupAddon>;

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

    return <Button {...button} xstyle={[styles.button, button.xstyle]} />;
}

/** Render text or an icon inside an addon. */
export function InputGroupText(
    properties: InputGroupElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    return renderPart("span", "input-group-text", properties, [text.sm, styles.text]);
}

/** The properties of a group's input besides its declared ones: its input's other attributes. */
export type InputGroupInputAttributes = Omit<
    InputProperties,
    "variant" | keyof typeof INPUT_GROUP_INPUT_PROPERTIES
>;

/** Render a group's input, plain inside its group's frame, tied to its field like any input. */
export const InputGroupInput = defineComponent({
    name: "inputGroupInput",
    properties: INPUT_GROUP_INPUT_PROPERTIES,
    render(
        properties: RenderProperties<
            InputGroupInputAttributes,
            typeof INPUT_GROUP_INPUT_PROPERTIES
        >,
    ): JSX.Element {
        return (
            <Input
                variant="plain"
                data-slot="input-group-control"
                {...properties}
                xstyle={[styles.control, controlMarker, properties.xstyle]}
            />
        );
    },
});

/** The properties of a group's input, the native input's attributes included. */
export type InputGroupInputProperties = ComponentProperties<typeof InputGroupInput>;

/** Render a group's textarea, tied to its field like any textarea. */
export function InputGroupTextarea(properties: TextareaProperties): JSX.Element {
    return (
        <Textarea
            variant="plain"
            data-slot="input-group-control"
            {...properties}
            xstyle={[
                styles.control,
                styles.textarea,
                controlMarker,
                multilineMarker,
                properties.xstyle,
            ]}
        />
    );
}
