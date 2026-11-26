import { state } from "@destack/theme/state";
import { Icon } from "@destack/ui/icon";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, motion, radius, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    type JSX,
    merge,
    omit,
    Show,
    useContext,
} from "@destack/view";
import { CheckboxGroupContext } from "./group.tsx";
import { useFieldControl, useFieldValue } from "../field/control.ts";
import { contentOr, type Render, renderPart } from "../part/index.ts";
import {
    ToggleInput,
    type ToggleFormProperties,
    type ToggleStateProperties,
    isSubmitted,
    ToggleState,
} from "../toggle-state/index.ts";
import { defineComponent } from "@destack/view/declare";
import { t } from "@destack/locale";
import { field } from "@destack/object";

/** A control beside a label with its help, whose first line it sits on. */
const BESIDE_CONTENT = ":is([data-slot=field]:has(> [data-slot=field-content]) > *)";

/** The styles of a checkbox and its indicator. */
const styles = style.create({
    checkbox: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        flexShrink: 0,
        marginBlockStart: { default: null, [BESIDE_CONTENT]: `calc((1lh - ${space[4]}) / 2)` },
        width: space[4],
        height: space[4],
        borderWidth: stroke.border,
        borderColor: color.mutedForeground,
        borderRadius: radius[2],
        color: swatch.accentLabel,
        cursor: {
            default: "pointer",
            ":disabled": "not-allowed",
            ":is([data-readonly])": "default",
        },
        transitionProperty: "background-color, border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    checked: {
        color: { default: swatch.accentLabel, [media.forcedColors]: "HighlightText" },
        forcedColorAdjust: { default: null, [media.forcedColors]: "none" },
        borderColor: { default: swatch.accentSolid, [media.forcedColors]: "Highlight" },
        backgroundColor: { default: swatch.accentSolid, [media.forcedColors]: "Highlight" },
    },
    indicator: {
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        width: "100%",
        height: "100%",
        color: "currentColor",
        pointerEvents: "none",
    },
});

/** The state of a checkbox, which its indicator reads. */
type CheckboxState = "checked" | "unchecked" | "indeterminate";

/** The state and availability of the nearest checkbox, which its indicator reads. */
interface CheckboxControl {
    /** Whether the box is checked, unchecked or neither. */
    readonly state: Accessor<CheckboxState>;
    /** Whether the box ignores the person. */
    readonly isDisabled: Accessor<boolean>;
}

/** The nearest checkbox, null outside one. */
const CheckboxContext = createContext<CheckboxControl | null>(null);

/** The properties of a checkbox, the native button's attributes included. */
export interface CheckboxProperties
    extends
        Omit<
            JSX.ButtonHTMLAttributes<HTMLButtonElement>,
            "class" | "type" | "role" | "name" | "value" | "form"
        >,
        ToggleStateProperties,
        ToggleFormProperties {
    /** Whether the box shows neither checked nor unchecked, such as for a partly selected list. */
    readonly indeterminate?: boolean;
    /** Whether the box checks every box of its group at once, in between while some are. */
    readonly parent?: boolean;
    /** The StyleX styles applied after the checkbox's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with the checkbox's attributes, the native button by default. */
    readonly render?: Render;
}

/** The properties of a checkbox's indicator, the native element's attributes included. */
export type CheckboxIndicatorProperties = Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class"> & {
    /** Whether the indicator stays rendered while the box is unchecked, for animating it. */
    readonly keepMounted?: boolean;
    /** The StyleX styles applied after the indicator's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
};

/** Edit the value of the nearest field's form. */
function useCheckboxField(given: CheckboxProperties): CheckboxProperties {
    return useFieldValue(given, {
        value: "checked",
        change: "onCheckedChange",
        read: (value) => value === true,
        settles: true,
    });
}

/** Render a button exposed as a checkbox around its indicator. */
export const Checkbox = defineComponent(
    "checkbox",
    function Checkbox(given: CheckboxProperties): JSX.Element {
        // follow the group around, else edit the value of the field's form
        const group = useContext(CheckboxGroupContext);
        const properties = group === null ? useCheckboxField(given) : group.checkbox(given);

        // take the id and state of the nearest fieldControl
        const nearest = useFieldControl();
        const fieldControl = group === null ? nearest : null;
        const toggle = new ToggleState(properties);
        const rest = omit(
            properties,
            "checked",
            "defaultChecked",
            "onCheckedChange",
            "readOnly",
            "indeterminate",
            "parent",
            "name",
            "value",
            "uncheckedValue",
            "required",
            "form",
            "disabled",
            "children",
        );
        const isDisabled = (): boolean =>
            properties.disabled === true || fieldControl?.isDisabled() === true;
        const isInvalid = (): boolean =>
            fieldControl?.isInvalid() === true || properties["aria-invalid"] === "true";
        const value = (): CheckboxState =>
            properties.indeterminate === true
                ? "indeterminate"
                : toggle.isChecked()
                  ? "checked"
                  : "unchecked";

        // mark the part, report its state and check it on a click or Space but not Enter
        return (
            <CheckboxContext value={{ state: value, isDisabled }}>
                {renderPart(
                    "button",
                    "checkbox",
                    rest,
                    () => [
                        state.focus,
                        state.touch,
                        state.disabled,
                        styles.checkbox,
                        value() !== "unchecked" && styles.checked,
                        isInvalid() && state.invalid,
                    ],
                    merge(
                        {
                            type: "button" as const,
                            role: "checkbox" as const,
                            get "aria-checked"() {
                                return value() === "indeterminate"
                                    ? "mixed"
                                    : value() === "checked"
                                      ? "true"
                                      : "false";
                            },
                            get "aria-readonly"() {
                                return properties.readOnly === true ? "true" : undefined;
                            },
                            get "data-readonly"() {
                                return properties.readOnly === true ? "" : undefined;
                            },
                            get "aria-required"() {
                                return properties.required === true ? "true" : undefined;
                            },
                            get "data-checked"() {
                                return value() === "checked" ? "" : undefined;
                            },
                            get "data-unchecked"() {
                                return value() === "unchecked" ? "" : undefined;
                            },
                            get "data-indeterminate"() {
                                return value() === "indeterminate" ? "" : undefined;
                            },
                            get "data-disabled"() {
                                return isDisabled() ? "" : undefined;
                            },
                            onClick: () => toggle.set(value() !== "checked"),
                            onKeyDown: (event: KeyboardEvent) => {
                                if (event.key === "Enter") {
                                    event.preventDefault();
                                }
                            },
                            get children() {
                                return contentOr(properties, () => <CheckboxIndicator />);
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
            </CheckboxContext>
        );
    },
    {
        indeterminate: field
            .boolean()
            .labeled(t.context("label")`Indeterminate`)
            .described(t`Shows a dash, as for a group of which some are checked.`)
            .default(false),
        disabled: field
            .boolean()
            .labeled(t.context("label")`Disabled`)
            .described(t`Refuses presses where the setting cannot change now.`)
            .default(false),
        readOnly: field
            .boolean()
            .labeled(t.context("label")`Read only`)
            .described(t`Shows the state, which a person cannot change.`)
            .default(false),
        required: field
            .boolean()
            .labeled(t.context("label")`Required`)
            .described(t`Asks for it to be checked before its form submits.`)
            .default(false),
    },
);

/** Render the mark of the nearest checkbox. */
export function CheckboxIndicator(properties: CheckboxIndicatorProperties): JSX.Element {
    // read the checkbox, refusing an indicator outside one
    const control = useContext(CheckboxContext);
    if (control === null) {
        throw new TypeError("a checkbox indicator needs a checkbox around it");
    }
    const rest = omit(properties, "keepMounted", "children");

    return (
        <Show when={properties.keepMounted === true || control.state() !== "unchecked"}>
            {renderPart("span", "checkbox-indicator", rest, styles.indicator, {
                get "data-checked"() {
                    return control.state() === "checked" ? "" : undefined;
                },
                get "data-indeterminate"() {
                    return control.state() === "indeterminate" ? "" : undefined;
                },
                get "data-disabled"() {
                    return control.isDisabled() ? "" : undefined;
                },
                get children() {
                    return contentOr(properties, () =>
                        control.state() === "indeterminate" ? (
                            <Icon name="minus" weight="bold" />
                        ) : (
                            <Icon name="check" weight="bold" />
                        ),
                    );
                },
            })}
        </Show>
    );
}
