import { state } from "@destack/theme/state";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import {
    color,
    motion,
    radius,
    shadow,
    size,
    space,
    stroke,
    swatch,
} from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    type JSX,
    merge,
    omit,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import {
    ToggleInput,
    type ToggleFormProperties,
    type ToggleStateProperties,
    isSubmitted,
    ToggleState,
} from "../toggle-state/index.ts";
import { useFieldControl, useFieldValue } from "../field/control.ts";
import { contentOr, type ElementPartProperties, type Render, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";
import { t } from "@destack/locale";
import { field } from "@destack/object";

/** The distance a checked thumb travels: the track's inner width less the thumb. */
const TRAVEL = `calc(${size[2]} - 2 * ${stroke.border} - ${space[4]})`;

/** The distance a small checked thumb travels. */
const SMALL_TRAVEL = `calc(${size[1]} - 2 * ${stroke.border} - ${space[3]})`;

/** The sizes of a switch. */
const SIZES = {
    sm: { title: t.context("option")`Small` },
    default: { title: t.context("option")`Default` },
};

/** The properties people change in a switch's controls and inspector. */
const SWITCH_PROPERTIES = {
    size: field
        .enum(SIZES)
        .labeled(t.context("label")`Size`)
        .described(t`How large the switch is beside its label.`)
        .default("default"),
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(
            t`Refuses presses where the setting cannot change now, as its field does when absent.`,
        )
        .optional(),
    readOnly: field
        .boolean()
        .labeled(t.context("label")`Read only`)
        .described(t`Shows the state, which a person cannot change.`)
        .default(false),
    required: field
        .boolean()
        .labeled(t.context("label")`Required`)
        .described(t`Asks for it to be on before its form submits.`)
        .default(false),
};

/** The styles of a switch and its thumb. */
const styles = style.create({
    switch: {
        forcedColorAdjust: { default: null, [media.forcedColors]: "none" },
        display: "inline-flex",
        alignItems: "center",
        flexShrink: 0,
        boxSizing: "content-box",
        width: `calc(${size[2]} - 2 * ${stroke.border})`,
        height: space[4],
        borderWidth: stroke.border,
        borderColor: { default: "transparent", [media.forcedColors]: "ButtonBorder" },
        borderRadius: radius.full,
        backgroundColor: { default: color.input, [media.forcedColors]: "ButtonFace" },
        cursor: {
            default: "pointer",
            ":disabled": "not-allowed",
            ":is([data-readonly])": "default",
        },
        transitionProperty: "background-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    checked: {
        backgroundColor: { default: swatch.accentSolid, [media.forcedColors]: "Highlight" },
    },
    thumb: {
        display: "block",
        flexShrink: 0,
        width: space[4],
        height: space[4],
        borderRadius: radius.full,
        backgroundColor: { default: "white", [media.forcedColors]: "ButtonText" },
        boxShadow: shadow.raised,
        pointerEvents: "none",
        transform: "none",
        transitionProperty: "transform",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    small: {
        width: `calc(${size[1]} - 2 * ${stroke.border})`,
        height: space[3],
    },
    smallThumb: {
        width: space[3],
        height: space[3],
    },
    smallThumbChecked: { transform: `translateX(${SMALL_TRAVEL})` },
    smallThumbCheckedReversed: { transform: `translateX(calc(-1 * ${SMALL_TRAVEL}))` },
    thumbChecked: {
        backgroundColor: { default: "white", [media.forcedColors]: "HighlightText" },
        transform: `translateX(${TRAVEL})`,
    },
    thumbCheckedReversed: {
        backgroundColor: { default: "white", [media.forcedColors]: "HighlightText" },
        transform: `translateX(calc(-1 * ${TRAVEL}))`,
    },
});

/** The checked state and availability of the nearest switch, which its thumb reads. */
interface SwitchControl {
    /** Whether the switch is on. */
    readonly isChecked: Accessor<boolean>;
    /** Whether the switch ignores the person. */
    readonly isDisabled: Accessor<boolean>;
    /** Whether the switch is small. */
    readonly isSmall: Accessor<boolean>;
}

/** The nearest switch, null outside one. */
const SwitchContext = createContext<SwitchControl | null>(null);

/** The properties of a switch besides its declared ones: the native button's attributes. */
export interface SwitchAttributes
    extends
        Omit<
            JSX.ButtonHTMLAttributes<HTMLButtonElement>,
            "class" | "type" | "role" | "name" | "value" | "form"
        >,
        ToggleStateProperties,
        ToggleFormProperties {
    /** The StyleX styles applied after the switch's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with the switch's attributes, the native button by default. */
    readonly render?: Render;
}

/** The properties of a switch's thumb, the native element's attributes included. */
export type SwitchThumbProperties = Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class"> &
    ElementPartProperties;

/** Render a button exposed as an on and off switch around its thumb. */
export const Switch = defineComponent({
    name: "switch",
    properties: SWITCH_PROPERTIES,
    render(given: RenderProperties<SwitchAttributes, typeof SWITCH_PROPERTIES>): JSX.Element {
        // edit the value of the field's form
        const properties = useFieldValue(given, {
            value: "checked",
            change: "onCheckedChange",
            read: (value) => value === true,
            settles: true,
        });

        // take the id and state of the nearest field
        const fieldControl = useFieldControl();
        const toggle = new ToggleState(properties);
        const rest = omit(
            properties,
            "checked",
            "defaultChecked",
            "onCheckedChange",
            "readOnly",
            "name",
            "value",
            "uncheckedValue",
            "required",
            "form",
            "disabled",
            "size",
            "children",
        );
        const isDisabled = (): boolean =>
            properties.disabled === true || fieldControl?.isDisabled() === true;

        // mark the part, report its state and flip it on activation
        return (
            <SwitchContext
                value={{
                    isChecked: toggle.isChecked,
                    isDisabled,
                    isSmall: () => properties.size === "sm",
                }}
            >
                {renderPart(
                    "button",
                    "switch",
                    rest,
                    () => [
                        state.focus,
                        state.touch,
                        state.disabled,
                        styles.switch,
                        properties.size === "sm" && styles.small,
                        toggle.isChecked() && styles.checked,
                    ],
                    merge(
                        {
                            type: "button" as const,
                            role: "switch" as const,
                            get "aria-checked"() {
                                return toggle.isChecked() ? "true" : "false";
                            },
                            get "aria-readonly"() {
                                return properties.readOnly ? "true" : undefined;
                            },
                            get "data-readonly"() {
                                return properties.readOnly ? "" : undefined;
                            },
                            get "aria-required"() {
                                return properties.required ? "true" : undefined;
                            },
                            get "data-checked"() {
                                return toggle.isChecked() ? "" : undefined;
                            },
                            get "data-unchecked"() {
                                return toggle.isChecked() ? undefined : "";
                            },
                            get "data-disabled"() {
                                return isDisabled() ? "" : undefined;
                            },
                            onClick: () => toggle.set(!toggle.isChecked()),
                            get children() {
                                return contentOr(properties, () => <SwitchThumb />);
                            },
                        },
                        fieldControl?.attributes() ?? {},
                        {
                            get disabled() {
                                return isDisabled() ? true : undefined;
                            },
                        },
                    ),
                )}
                <Show when={isSubmitted(properties)}>
                    <ToggleInput
                        name={properties.name}
                        value={properties.value}
                        uncheckedValue={properties.uncheckedValue}
                        required={properties.required}
                        form={properties.form}
                        checked={toggle.isChecked()}
                        disabled={isDisabled()}
                        onReset={() => toggle.reset()}
                    />
                </Show>
            </SwitchContext>
        );
    },
});

/** The properties of a switch, the native button's attributes included. */
export type SwitchProperties = ComponentProperties<typeof Switch>;

/** Render the thumb of the nearest switch, which slides to the far end while the switch is on. */
export function SwitchThumb(properties: SwitchThumbProperties): JSX.Element {
    // read the switch and the text direction the thumb slides along
    const control = useContext(SwitchContext);
    if (control === null) {
        throw new TypeError("a switch thumb needs a switch around it");
    }
    const locale = useLocale();

    return renderPart(
        "span",
        "switch-thumb",
        properties,
        () => [
            styles.thumb,
            control.isSmall() && styles.smallThumb,
            control.isChecked() &&
                (locale.direction === "rtl" ? styles.thumbCheckedReversed : styles.thumbChecked),
            control.isChecked() &&
                control.isSmall() &&
                (locale.direction === "rtl"
                    ? styles.smallThumbCheckedReversed
                    : styles.smallThumbChecked),
        ],
        {
            get "data-checked"() {
                return control.isChecked() ? "" : undefined;
            },
            get "data-unchecked"() {
                return control.isChecked() ? undefined : "";
            },
            get "data-disabled"() {
                return control.isDisabled() ? "" : undefined;
            },
        },
    );
}
