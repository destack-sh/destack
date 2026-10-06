import * as style from "@destack/style";
import { color, motion, radius, shadow, space, stroke } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createUniqueId,
    type JSX,
    omit,
    useContext,
} from "@destack/view";

/** The checked value of the nearest radio group, null outside a group. */
const RadioGroupContext = createContext<RadioGroupControl | null>(null);

/** The styles of a radio group and its radios. */
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
        appearance: "none",
        position: "relative",
        flexShrink: 0,
        width: space[4],
        height: space[4],
        margin: 0,
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: { default: color.input, ":checked": color.primary },
        borderRadius: radius.full,
        backgroundColor: "transparent",
        boxShadow: shadow.inset,
        cursor: { default: "pointer", ":disabled": "not-allowed" },
        opacity: { default: 1, ":disabled": 0.5 },
        transitionProperty: "border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
        "::after": {
            content: '""',
            position: "absolute",
            inset: "25%",
            borderRadius: radius.full,
            backgroundColor: color.primary,
            opacity: { default: 0, ":checked": 1 },
        },
    },
});

/** The properties of a radio group, the native element's attributes included. */
export interface RadioGroupProperties extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The form field name the radios share, a generated one by default. */
    readonly name?: string;
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
    /** The direction the radios lay out along, vertical by default; arrow keys move both ways natively. */
    readonly orientation?: "horizontal" | "vertical";
    /** The StyleX styles applied after the group's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of a radio, the native radio's attributes included. */
export interface RadioGroupItemProperties extends Omit<
    JSX.InputHTMLAttributes<HTMLInputElement>,
    "class" | "type" | "name" | "value" | "checked" | "onChange"
> {
    /** The value the radio stands for. */
    readonly value: string;
    /** The StyleX styles applied after the radio's styles. */
    readonly xstyle?: style.Styles;
}

/** The checked value of a radio group, which its radios share. */
interface RadioGroupControl {
    /** The form field name the radios share. */
    readonly name: string;
    /** The checked value. */
    readonly value: Accessor<string | undefined>;
    /** Check a value as the person chose it, holding a controlled group at its owner's value. */
    readonly choose: (value: string, radio: HTMLInputElement) => void;
    /** The group's properties, read for its availability. */
    readonly properties: RadioGroupProperties;
}

/** Render a group of native radios that share a name, which arrow keys move between. */
export function RadioGroup(properties: RadioGroupProperties): JSX.Element {
    // follow the controlled value or the group's own
    const rest = omit(
        properties,
        "name",
        "value",
        "defaultValue",
        "onValueChange",
        "disabled",
        "required",
        "orientation",
        "xstyle",
        "style",
    );
    const [value, setValue] = createControllableSignal<string | undefined>({
        isControlled: () => "value" in properties,
        value: () => properties.value,
        defaultValue: properties.defaultValue,
        onChange: (next) => next !== undefined && properties.onValueChange?.(next),
    });
    const control: RadioGroupControl = {
        name: properties.name ?? createUniqueId(),
        value,
        properties,
        choose: (next, radio) => {
            // keep the choice and show the owner's value until the owner takes it
            setValue(next);
            if ("value" in properties) {
                for (const sibling of radio
                    .closest("[data-slot=radio-group]")
                    ?.querySelectorAll<HTMLInputElement>("input[type=radio]") ?? []) {
                    sibling.checked = sibling.value === properties.value;
                }
            }
        },
    };

    return (
        <RadioGroupContext value={control}>
            <div
                role="radiogroup"
                data-slot="radio-group"
                aria-orientation={properties.orientation}
                aria-required={properties.required === true ? "true" : undefined}
                aria-disabled={properties.disabled === true ? "true" : undefined}
                {...rest}
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

/** Render a native radio of the nearest radio group, checked while its value is the group's. */
export function RadioGroupItem(properties: RadioGroupItemProperties): JSX.Element {
    // read the group and refuse a radio outside one
    const group = useContext(RadioGroupContext);
    if (group === null) {
        throw new TypeError("a radio group item needs a radio group around it");
    }
    const rest = omit(properties, "value", "disabled", "xstyle", "style");

    return (
        <input
            type="radio"
            name={group.name}
            value={properties.value}
            checked={group.value() === properties.value}
            disabled={properties.disabled === true || group.properties.disabled === true}
            required={group.properties.required === true}
            data-slot="radio-group-item"
            {...rest}
            onChange={(event) => group.choose(properties.value, event.currentTarget)}
            {...style.attributes([styles.item, properties.xstyle], properties.style)}
        />
    );
}
