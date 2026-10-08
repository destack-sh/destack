import * as style from "@destack/style";
import { color, motion, radius, shadow, space, stroke } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createUniqueId,
    type JSX,
    merge,
    omit,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import { ToggleInput, isSubmitted } from "../toggle-state/index.ts";
import { ListState } from "../focus/index.ts";
import { Selection, type SingleSelection, valuesOf } from "../selection/index.ts";
import { type PartAttributes, type Render, rendered } from "../part/index.ts";

/** The checked value of the nearest radio group, null outside a group. */
const RadioGroupContext = createContext<RadioGroupControl | null>(null);

/** The checked state of the nearest radio, null outside one. */
const RadioGroupItemContext = createContext<RadioGroupItemControl | null>(null);

/** The styles of a radio group, its radios and their indicators. */
const styles = style.create({
    group: {
        display: "grid",
        gap: space[3],
    },
    horizontal: {
        gridAutoFlow: "column",
        justifyContent: "start",
    },
    item: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        flexShrink: 0,
        width: space[4],
        height: space[4],
        borderWidth: stroke.border,
        borderColor: color.input,
        borderRadius: radius.full,
        boxShadow: shadow.inset,
        cursor: { default: "pointer", ":disabled": "not-allowed" },
        opacity: { default: 1, ":disabled": 0.5 },
        transitionProperty: "border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
    checked: {
        borderColor: color.primary,
    },
    indicator: {
        display: "block",
        width: "50%",
        height: "50%",
        borderRadius: radius.full,
        backgroundColor: color.primary,
        pointerEvents: "none",
    },
});

/** The properties of a radio group, the native element's attributes included. */
export interface RadioGroupProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "onKeyDown"
> {
    /** The form field name the radios submit under, which renders a hidden native radio per radio. */
    readonly name?: string;
    /** The id of the form the radios belong to, their ancestor form by default. */
    readonly form?: string;
    /** The checked value, which makes the state controlled, undefined for none. */
    readonly value?: string | undefined;
    /** The value checked at first while uncontrolled. */
    readonly defaultValue?: string;
    /** Handle the person checking a radio. */
    readonly onValueChange?: (value: string) => void;
    /** Whether every radio ignores the person. */
    readonly disabled?: boolean;
    /** Whether a form needs a checked radio before it submits. */
    readonly required?: boolean;
    /** The direction the radios lay out along; arrow keys move both ways either way. */
    readonly orientation?: "horizontal" | "vertical";
    /** The StyleX styles applied after the group's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of a radio, the native button's attributes included. */
export interface RadioGroupItemProperties extends Omit<
    JSX.ButtonHTMLAttributes<HTMLButtonElement>,
    "class" | "type" | "role" | "name" | "value" | "form" | "onClick"
> {
    /** The value the radio stands for. */
    readonly value: string;
    /** The StyleX styles applied after the radio's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with the radio's attributes, the native button by default. */
    readonly render?: Render;
}

/** The properties of a radio's indicator, the native element's attributes included. */
export type RadioGroupIndicatorProperties = Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class"> & {
    /** Whether the indicator stays rendered while its radio is unchecked, for animating it. */
    readonly forceMount?: boolean;
    /** The StyleX styles applied after the indicator's styles. */
    readonly xstyle?: style.Styles;
};

/** The checked value and radios of a radio group, which its radios share. */
class RadioGroupControl {
    /** The group's properties, read for its availability and form. */
    readonly properties: RadioGroupProperties;
    /** The name the hidden native radios share, the group's or a generated one. */
    readonly name: string;
    /** The checked value, one at most. */
    readonly selection: Selection;
    /** The radios in document order and the one holding the tab stop. */
    readonly list: ListState;

    /** Create the state of a radio group from its root's properties. */
    constructor(properties: RadioGroupProperties) {
        // follow the controlled value or the group's own, reporting each checked value
        const onValueChange = (next: string | undefined): void => {
            if (next !== undefined) {
                properties.onValueChange?.(next);
            }
        };
        this.properties = properties;
        this.name = properties.name ?? createUniqueId();
        this.selection = new Selection(
            "value" in properties
                ? {
                      get value() {
                          return properties.value;
                      },
                      onValueChange,
                  }
                : { ...defaultOf(properties.defaultValue), onValueChange },
        );

        // hold the tab stop on the checked radio, else the first available one, moving on every arrow
        this.list = new ListState({
            orientation: "both",
            isLooping: true,
            isTypeahead: false,
            initial: () => this.value(),
        });
    }

    /** The checked value, undefined for none. */
    value(): string | undefined {
        return this.selection.values()[0];
    }

    /** Check a radio's value. */
    choose(value: string): void {
        this.selection.select(value);
    }

    /** Return the group to the value it started with, as a form reset does. */
    reset(): void {
        this.selection.replace(valuesOf(this.properties.defaultValue));
    }
}

/** The checked state and availability of a radio, which its indicator reads. */
interface RadioGroupItemControl {
    /** Whether the radio is checked. */
    readonly isChecked: Accessor<boolean>;
    /** Whether the radio ignores the person. */
    readonly isDisabled: Accessor<boolean>;
}

/** Read the default of a single selection, none when absent. */
function defaultOf(value: string | undefined): Pick<SingleSelection, "defaultValue"> {
    return value === undefined ? {} : { defaultValue: value };
}

/** Render a group of radios, which arrow keys move between and check, and a form submits through hidden native radios. */
export function RadioGroup(properties: RadioGroupProperties): JSX.Element {
    // share the group with its radios and read the text direction for the arrow keys
    const control = new RadioGroupControl(properties);
    const locale = useLocale();
    const rest = omit(
        properties,
        "name",
        "form",
        "value",
        "defaultValue",
        "onValueChange",
        "disabled",
        "required",
        "orientation",
        "xstyle",
        "style",
    );

    return (
        <RadioGroupContext value={control}>
            <div
                role="radiogroup"
                data-slot="radio-group"
                data-orientation={properties.orientation}
                data-disabled={properties.disabled === true ? "" : undefined}
                aria-orientation={properties.orientation}
                aria-required={properties.required === true ? "true" : undefined}
                aria-disabled={properties.disabled === true ? "true" : undefined}
                {...rest}
                onKeyDown={(event) => {
                    // move to the next or previous radio and check it
                    const target = control.list.focus.move(event, locale.direction);
                    if (target !== undefined) {
                        control.choose(target);
                    }
                }}
                onFocusOut={(event) => control.list.focus.focusOut(event)}
                {...style.attributes(
                    [
                        styles.group,
                        properties.orientation === "horizontal" && styles.horizontal,
                        properties.xstyle,
                    ],
                    properties.style,
                )}
            />
        </RadioGroupContext>
    );
}

/** Render a radio of the nearest radio group around its indicator, checked while its value is the group's. */
export function RadioGroupItem(properties: RadioGroupItemProperties): JSX.Element {
    // read the group and refuse a radio outside one
    const group = useContext(RadioGroupContext);
    if (group === null) {
        throw new TypeError("a radio group item needs a radio group around it");
    }
    const rest = omit(properties, "value", "disabled", "xstyle", "style", "render", "children");
    const isDisabled = (): boolean =>
        properties.disabled === true || group.properties.disabled === true;
    const isChecked = (): boolean => group.selection.isSelected(properties.value);
    let element: HTMLElement | undefined;
    group.list.add({
        key: properties.value,
        text: () => properties.value,
        isDisabled,
        element: () => element,
    });

    // mark the part, report its state and check it on a click or Space
    const part: PartAttributes = merge(
        {
            role: "radio" as const,
            get "aria-checked"() {
                return isChecked() ? "true" : "false";
            },
            get tabindex() {
                return group.list.focus.isActive(properties.value) ? 0 : -1;
            },
            get disabled() {
                return isDisabled() ? true : undefined;
            },
            ref: (target: HTMLElement) => {
                element = target;
            },
            "data-slot": "radio-group-item",
            get "data-state"() {
                return isChecked() ? "checked" : "unchecked";
            },
            get "data-disabled"() {
                return isDisabled() ? "" : undefined;
            },
            get "data-value"() {
                return properties.value;
            },
            onClick: () => group.choose(properties.value),
            onFocus: () => group.list.focus.focusIn(properties.value),
            get children() {
                return properties.children ?? <RadioGroupIndicator />;
            },
        },
        () =>
            style.attributes(
                [styles.item, isChecked() && styles.checked, properties.xstyle],
                properties.style,
            ),
    );

    return (
        <RadioGroupItemContext value={{ isChecked, isDisabled }}>
            {rendered(properties.render, part, rest, () => (
                <button type="button" {...part} {...rest} />
            ))}
            <Show when={isSubmitted(group.properties)}>
                <ToggleInput
                    type="radio"
                    name={group.name}
                    value={properties.value}
                    required={group.properties.required}
                    form={group.properties.form}
                    checked={isChecked()}
                    disabled={isDisabled()}
                    onReset={() => group.reset()}
                />
            </Show>
        </RadioGroupItemContext>
    );
}

/** Render the dot of the nearest radio, shown while it is checked. */
export function RadioGroupIndicator(properties: RadioGroupIndicatorProperties): JSX.Element {
    // read the radio, refusing an indicator outside one
    const item = useContext(RadioGroupItemContext);
    if (item === null) {
        throw new TypeError("a radio group indicator needs a radio group item around it");
    }
    const rest = omit(properties, "forceMount", "xstyle", "style");

    return (
        <Show when={properties.forceMount === true || item.isChecked()}>
            <span
                data-slot="radio-group-indicator"
                data-state={item.isChecked() ? "checked" : "unchecked"}
                data-disabled={item.isDisabled() ? "" : undefined}
                {...rest}
                {...style.attributes([styles.indicator, properties.xstyle], properties.style)}
            />
        </Show>
    );
}
