import { state } from "@destack/theme/state";
import { AXES } from "../position/index.ts";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, motion, radius, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createUniqueId,
    type JSX,
    omit,
    Show,
    useContext,
    useLocale,
    untrack,
} from "@destack/view";
import { ToggleInput, isSubmitted } from "../toggle-state/index.ts";
import { ListState } from "../focus/index.ts";
import { Selection, type SingleSelection, valuesOf } from "../selection/index.ts";
import { contentOr, type Render, renderPart } from "../part/index.ts";
import { useFieldValue, readChoice } from "../field/control.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The checked value of the nearest radio group, null outside a group. */
const RadioGroupContext = createContext<RadioGroupControl | null>(null);

/** Read the nearest radio group, refusing a radio outside one. */
export function useRadioGroup(): RadioGroupControl {
    const group = useContext(RadioGroupContext);
    if (group === null) {
        throw new TypeError("a radio group item needs a radio group around it");
    }

    return group;
}

/** The checked state of the nearest radio, null outside one. */
const RadioGroupItemContext = createContext<RadioGroupItemControl | null>(null);

/** A control beside a label with its help, whose first line it sits on. */
const BESIDE_CONTENT = ":is([data-slot=field]:has(> [data-slot=field-content]) > *)";

/** The properties people change in a radio group's controls and inspector. */
const RADIO_GROUP_PROPERTIES = {
    orientation: field
        .enum(AXES)
        .labeled(t.context("label")`Orientation`)
        .described(t`Whether the radios lie in a column or a row.`)
        .default("vertical"),
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses presses on every radio of the group.`)
        .default(false),
    readOnly: field
        .boolean()
        .labeled(t.context("label")`Read only`)
        .described(t`Shows the checked radio, which a person cannot change.`)
        .default(false),
    required: field
        .boolean()
        .labeled(t.context("label")`Required`)
        .described(t`Asks for a checked radio before its form submits.`)
        .default(false),
};

/** The properties people change in a radio's controls and inspector. */
const RADIO_GROUP_ITEM_PROPERTIES = {
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses presses on this radio alone.`)
        .default(false),
};

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
        marginBlockStart: { default: null, [BESIDE_CONTENT]: `calc((1lh - ${space[4]}) / 2)` },
        width: space[4],
        height: space[4],
        borderWidth: stroke.border,
        borderColor: color.mutedForeground,
        borderRadius: radius.full,
        cursor: { default: "pointer", ":disabled": "not-allowed" },
        transitionProperty: "border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    checked: {
        borderColor: swatch.accentSolid,
    },
    indicator: {
        display: "block",
        width: "50%",
        height: "50%",
        borderRadius: radius.full,
        backgroundColor: { default: swatch.accentSolid, [media.forcedColors]: "Highlight" },
        pointerEvents: "none",
    },
});

/** The properties of a radio group besides its declared ones: its native attributes and form. */
export interface RadioGroupAttributes extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The form field name the radios submit under. */
    readonly name?: string;
    /** The id of the form the radios belong to, their ancestor form by default. */
    readonly form?: string;
    /** The checked value, which makes the state controlled, undefined for none. */
    readonly value?: string | undefined;
    /** The value checked at first while uncontrolled. */
    readonly defaultValue?: string;
    /** Handle the person checking a radio. */
    readonly onValueChange?: (value: string) => void;
    /** The StyleX styles applied after the group's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
}

/** The properties of a radio besides its declared ones: the native button's attributes. */
export interface RadioGroupItemAttributes extends Omit<
    JSX.ButtonHTMLAttributes<HTMLButtonElement>,
    "class" | "type" | "role" | "name" | "value" | "form" | "disabled"
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
    readonly keepMounted?: boolean;
    /** The StyleX styles applied after the indicator's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
};

/** The checked value and radios of a radio group, which its radios share. */
export class RadioGroupControl {
    /** The group's properties, read for its availability and form. */
    readonly properties: RenderProperties<RadioGroupAttributes, typeof RADIO_GROUP_PROPERTIES>;
    /** The name the hidden native radios share, the group's or a generated one. */
    readonly name: string;
    /** The checked value, one at most. */
    readonly selection: Selection;
    /** The radios in document order and the one with the tab stop. */
    readonly list: ListState;

    /** Create the state of a radio group from its root's properties. */
    constructor(properties: RenderProperties<RadioGroupAttributes, typeof RADIO_GROUP_PROPERTIES>) {
        // follow the controlled value or the group's own, reporting each checked value
        const onValueChange = (next: string | undefined): void => {
            if (next !== undefined) {
                properties.onValueChange?.(next);
            }
        };
        this.properties = properties;
        this.name = untrack(() => properties.name) ?? createUniqueId();
        this.selection = new Selection(
            "value" in properties
                ? {
                      get value() {
                          return properties.value;
                      },
                      onValueChange,
                  }
                : { ...defaultOf(untrack(() => properties.defaultValue)), onValueChange },
        );

        // put the tab stop on the checked radio, else the first available one, moving on every arrow
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
        if (!this.properties.readOnly) {
            this.selection.add(value);
        }
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

/** Render a group of radios, which arrow keys move between and check. */
export const RadioGroup = defineComponent({
    name: "radioGroup",
    properties: RADIO_GROUP_PROPERTIES,
    render(
        given: RenderProperties<RadioGroupAttributes, typeof RADIO_GROUP_PROPERTIES>,
    ): JSX.Element {
        // edit the value of the field's form
        const properties = useFieldValue(given, {
            value: "value",
            change: "onValueChange",
            read: readChoice,
            settles: true,
        });

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
            "readOnly",
            "required",
            "orientation",
        );

        return (
            <RadioGroupContext value={control}>
                {renderPart(
                    "div",
                    "radio-group",
                    rest,
                    () => [
                        styles.group,
                        properties.orientation === "horizontal" && styles.horizontal,
                    ],
                    {
                        role: "radiogroup" as const,
                        get "data-orientation"() {
                            return properties.orientation;
                        },
                        get "data-disabled"() {
                            return properties.disabled ? "" : undefined;
                        },
                        get "aria-orientation"() {
                            return properties.orientation;
                        },
                        get "data-readonly"() {
                            return properties.readOnly ? "" : undefined;
                        },
                        get "aria-readonly"() {
                            return properties.readOnly ? "true" : undefined;
                        },
                        get "aria-required"() {
                            return properties.required ? "true" : undefined;
                        },
                        get "aria-disabled"() {
                            return properties.disabled ? "true" : undefined;
                        },
                        onKeyDown: (event) => {
                            // move to the next or previous radio and check it
                            const target = control.list.focus.move(event, locale.direction);
                            if (target !== undefined) {
                                control.choose(target);
                            }
                        },
                        onFocusOut: (event) => control.list.focus.focusOut(event),
                    },
                )}
            </RadioGroupContext>
        );
    },
});

/** The properties of a radio group, its native attributes included. */
export type RadioGroupProperties = ComponentProperties<typeof RadioGroup>;

/** Render a radio of the nearest radio group around its indicator. */
export const RadioGroupItem = defineComponent({
    name: "radioGroupItem",
    properties: RADIO_GROUP_ITEM_PROPERTIES,
    render(
        properties: RenderProperties<RadioGroupItemAttributes, typeof RADIO_GROUP_ITEM_PROPERTIES>,
    ): JSX.Element {
        // read the group and refuse a radio outside one
        const group = useRadioGroup();
        const rest = omit(properties, "value", "disabled", "children");
        const isDisabled = (): boolean => properties.disabled || group.properties.disabled;
        const isChecked = (): boolean => group.selection.isSelected(properties.value);
        let element: HTMLElement | undefined;
        group.list.add({
            key: properties.value,
            text: () => properties.value,
            isDisabled,
            element: () => element,
        });

        // mark the part, report its state and check it on a click or Space
        return (
            <RadioGroupItemContext value={{ isChecked, isDisabled }}>
                {renderPart(
                    "button",
                    "radio-group-item",
                    rest,
                    () => [
                        state.focus,
                        state.touch,
                        state.disabled,
                        styles.item,
                        isChecked() && styles.checked,
                    ],
                    {
                        type: "button" as const,
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
                        get "data-checked"() {
                            return isChecked() ? "" : undefined;
                        },
                        get "data-unchecked"() {
                            return isChecked() ? undefined : "";
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
                            return contentOr(properties, () => <RadioGroupIndicator />);
                        },
                    },
                )}
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
    },
});

/** The properties of a radio, the native button's attributes included. */
export type RadioGroupItemProperties = ComponentProperties<typeof RadioGroupItem>;

/** Render the dot of the nearest radio, shown while it is checked. */
export function RadioGroupIndicator(properties: RadioGroupIndicatorProperties): JSX.Element {
    // read the radio, refusing an indicator outside one
    const item = useContext(RadioGroupItemContext);
    if (item === null) {
        throw new TypeError("a radio group indicator needs a radio group item around it");
    }
    const rest = omit(properties, "keepMounted");

    return (
        <Show when={properties.keepMounted === true || item.isChecked()}>
            {renderPart("span", "radio-group-indicator", rest, styles.indicator, {
                get "data-checked"() {
                    return item.isChecked() ? "" : undefined;
                },
                get "data-unchecked"() {
                    return item.isChecked() ? undefined : "";
                },
                get "data-disabled"() {
                    return item.isDisabled() ? "" : undefined;
                },
            })}
        </Show>
    );
}
