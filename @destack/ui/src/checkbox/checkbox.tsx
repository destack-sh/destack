import { Icon } from "@destack/icon";
import * as style from "@destack/style";
import { color, motion, radius, shadow, space, stroke } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    type JSX,
    merge,
    omit,
    Show,
    useContext,
} from "@destack/view";
import { useFieldControl } from "../field/control.ts";
import { type PartAttributes, type Render, rendered } from "../part/index.ts";
import {
    ToggleInput,
    type ToggleFormProperties,
    type ToggleStateProperties,
    isSubmitted,
    ToggleState,
} from "../toggle-state/index.ts";

/** The styles of a checkbox and its indicator. */
const styles = style.create({
    checkbox: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        flexShrink: 0,
        width: space[4],
        height: space[4],
        borderWidth: stroke.border,
        borderColor: color.input,
        borderRadius: radius[2],
        color: color.primaryForeground,
        boxShadow: shadow.inset,
        cursor: { default: "pointer", ":disabled": "not-allowed" },
        opacity: { default: 1, ":disabled": 0.5 },
        transitionProperty: "background-color, border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
    checked: {
        borderColor: color.primary,
        backgroundColor: color.primary,
    },
    invalid: {
        borderColor: color.destructive,
        outlineColor: `color-mix(in oklab, ${color.destructive} 20%, transparent)`,
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
            "class" | "type" | "role" | "name" | "value" | "form" | "onClick" | "onKeyDown"
        >,
        ToggleStateProperties,
        ToggleFormProperties {
    /** Whether the box shows neither checked nor unchecked, such as for a partly selected list. */
    readonly indeterminate?: boolean;
    /** The StyleX styles applied after the checkbox's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with the checkbox's attributes, the native button by default. */
    readonly render?: Render;
}

/** The properties of a checkbox's indicator, the native element's attributes included. */
export type CheckboxIndicatorProperties = Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class"> & {
    /** Whether the indicator stays rendered while the box is unchecked, for animating it. */
    readonly forceMount?: boolean;
    /** The StyleX styles applied after the indicator's styles. */
    readonly xstyle?: style.Styles;
};

/** Render a button exposed as a checkbox around its indicator, which a click or Space toggles and a form submits through a hidden checkbox. */
export function Checkbox(properties: CheckboxProperties): JSX.Element {
    // take the id and state of the nearest field
    const field = useFieldControl();
    const toggle = new ToggleState(properties);
    const rest = omit(
        properties,
        "checked",
        "defaultChecked",
        "onCheckedChange",
        "indeterminate",
        "name",
        "value",
        "required",
        "form",
        "disabled",
        "xstyle",
        "style",
        "render",
        "children",
    );
    const isDisabled = (): boolean => properties.disabled === true || field?.isDisabled() === true;
    const isInvalid = (): boolean =>
        field?.isInvalid() === true || properties["aria-invalid"] === "true";
    const state = (): CheckboxState =>
        properties.indeterminate === true
            ? "indeterminate"
            : toggle.isChecked()
              ? "checked"
              : "unchecked";

    // mark the part, report its state and check it on a click or Space but not Enter
    const part: PartAttributes = merge(
        {
            role: "checkbox" as const,
            get "aria-checked"() {
                return state() === "indeterminate" ? "mixed" : String(state() === "checked");
            },
            get "aria-required"() {
                return properties.required === true ? "true" : undefined;
            },
            "data-slot": "checkbox",
            get "data-state"() {
                return state();
            },
            get "data-disabled"() {
                return isDisabled() ? "" : undefined;
            },
            onClick: () => toggle.set(state() !== "checked"),
            onKeyDown: (event: KeyboardEvent) => {
                if (event.key === "Enter") {
                    event.preventDefault();
                }
            },
            get children() {
                return properties.children ?? <CheckboxIndicator />;
            },
        },
        () => ({ ...field?.attributes(), disabled: isDisabled() ? true : undefined }),
        () =>
            style.attributes(
                [
                    styles.checkbox,
                    state() !== "unchecked" && styles.checked,
                    isInvalid() && styles.invalid,
                    properties.xstyle,
                ],
                properties.style,
            ),
    );

    return (
        <CheckboxContext value={{ state, isDisabled }}>
            {rendered(properties.render, part, rest, () => (
                <button type="button" {...part} {...rest} />
            ))}
            <Show when={isSubmitted(properties)}>
                <ToggleInput
                    name={properties.name}
                    value={properties.value}
                    required={properties.required}
                    form={properties.form}
                    checked={toggle.isChecked()}
                    disabled={isDisabled()}
                    onReset={() => toggle.reset()}
                />
            </Show>
        </CheckboxContext>
    );
}

/** Render the mark of the nearest checkbox: a check while checked and a dash while indeterminate. */
export function CheckboxIndicator(properties: CheckboxIndicatorProperties): JSX.Element {
    // read the checkbox, refusing an indicator outside one
    const control = useContext(CheckboxContext);
    if (control === null) {
        throw new TypeError("a checkbox indicator needs a checkbox around it");
    }
    const rest = omit(properties, "forceMount", "xstyle", "style", "children");

    return (
        <Show when={properties.forceMount === true || control.state() !== "unchecked"}>
            <span
                data-slot="checkbox-indicator"
                data-state={control.state()}
                data-disabled={control.isDisabled() ? "" : undefined}
                {...rest}
                {...style.attributes([styles.indicator, properties.xstyle], properties.style)}
            >
                {properties.children ??
                    (control.state() === "indeterminate" ? (
                        <Icon name="minus" weight="bold" />
                    ) : (
                        <Icon name="check" weight="bold" />
                    ))}
            </span>
        </Show>
    );
}
