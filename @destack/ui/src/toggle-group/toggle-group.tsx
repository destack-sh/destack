import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { useLocale } from "@destack/locale/solid";
import type { JSX } from "@solidjs/web";
import {
    createContext,
    createSignal,
    merge,
    omit,
    onCleanup,
    useContext,
    type Accessor,
    type Setter,
} from "solid-js";
import { directionOf, itemsOf, moveFocus, type Orientation } from "../focus/index.ts";
import { toggleStyle, type ToggleSize, type ToggleVariant } from "../toggle/index.ts";

/** The orientation of a toggle group that sets none. */
const DEFAULTS: Required<Pick<ToggleGroupProperties, "orientation">> = {
    orientation: "horizontal",
};

/** The group of the nearest toggle group, null outside one. */
const ToggleGroupContext = createContext<ToggleGroupControl | null>(null);

/** The layout of a toggle group in each orientation. */
const orientations = style.create({
    horizontal: { display: "flex", alignItems: "center", gap: space[1] },
    vertical: { display: "flex", flexDirection: "column", gap: space[1] },
});

/** Whether one or several items of a toggle group are on at a time. */
export type ToggleGroupType = "single" | "multiple";

/** The pressed values and focus of a toggle group, which its items share. */
export class ToggleGroupControl {
    /** The properties of the group root, read for its controlled state and look. */
    readonly properties: ToggleGroupProperties;
    /** The values of the items in document order. */
    readonly values: Accessor<readonly string[]>;
    /** The value of the item the focus last rested on. */
    readonly focused: Accessor<string | undefined>;
    /** The pressed values when uncontrolled. */
    readonly #ownValue: Accessor<readonly string[]>;
    /** Replace the pressed values when uncontrolled. */
    readonly #setValue: Setter<readonly string[]>;
    /** Replace the values of the items. */
    readonly #setValues: Setter<readonly string[]>;
    /** Replace the value of the focused item. */
    readonly #setFocused: Setter<string | undefined>;

    /** Create the state of a toggle group from its root's properties. */
    constructor(properties: ToggleGroupProperties) {
        // start from the default value with no items and no focus
        const [ownValue, setValue] = createSignal(pressedOf(properties.defaultValue));
        const [values, setValues] = createSignal<readonly string[]>([], { ownedWrite: true });
        const [focused, setFocused] = createSignal<string | undefined>(undefined);
        this.properties = properties;
        this.values = values;
        this.focused = focused;
        this.#ownValue = ownValue;
        this.#setValue = setValue;
        this.#setValues = setValues;
        this.#setFocused = setFocused;
    }

    /** Read the pressed values, controlled or the group's own. */
    value(): readonly string[] {
        return "value" in this.properties ? pressedOf(this.properties.value) : this.#ownValue();
    }

    /** Add an item's value until the item unmounts. */
    register(value: string): void {
        this.#setValues((values) => [...values, value]);
        onCleanup(() => this.#setValues((values) => values.filter((entry) => entry !== value)));
    }

    /** Remember the item the focus rests on. */
    focus(value: string): void {
        this.#setFocused(value);
    }

    /** Turn an item on or off, turning the others off in a single group, and tell the change handler. */
    toggle(value: string): void {
        // compute the pressed values after the toggle
        const current = this.value();
        const isPressed = current.includes(value);
        const next = isPressed
            ? current.filter((entry) => entry !== value)
            : this.properties.type === "multiple"
              ? [...current, value]
              : [value];

        // keep and report them, a single group's as one value or none
        this.#setValue(next);
        const properties = this.properties;
        if (properties.type === "single") {
            properties.onValueChange?.(next[0]);
        } else {
            properties.onValueChange?.(next);
        }
    }

    /** Report whether an item takes the tab stop: the focused one, else the first pressed, else the first. */
    isTabStop(value: string): boolean {
        const values = this.values();
        const stop =
            this.focused() ?? values.find((entry) => this.value().includes(entry)) ?? values[0];

        return stop === value;
    }
}

/** The pressed value of a single toggle group, one value or none. */
export interface ToggleGroupSingle {
    /** One item is on at a time. */
    readonly type: "single";
    /** The pressed value, which makes the state controlled, undefined for none. */
    readonly value?: string | undefined;
    /** The value pressed at first when the state is uncontrolled. */
    readonly defaultValue?: string;
    /** Handle the pressed value changing, undefined once the pressed item turns off. */
    readonly onValueChange?: (value: string | undefined) => void;
}

/** The pressed values of a multiple toggle group. */
export interface ToggleGroupMultiple {
    /** Several items are on at a time. */
    readonly type: "multiple";
    /** The pressed values, which make the state controlled. */
    readonly value?: readonly string[];
    /** The values pressed at first when the state is uncontrolled. */
    readonly defaultValue?: readonly string[];
    /** Handle the pressed values changing. */
    readonly onValueChange?: (value: readonly string[]) => void;
}

/** The properties of a toggle group, the native element's attributes included. */
export type ToggleGroupProperties = ToggleGroupLook & (ToggleGroupSingle | ToggleGroupMultiple);

/** The look and layout of a toggle group, the native element's attributes included. */
export interface ToggleGroupLook extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style" | "onKeyDown"
> {
    /** The look of every item, default by default. */
    readonly variant?: ToggleVariant;
    /** The height and padding of every item, default by default. */
    readonly size?: ToggleSize;
    /** The direction the arrow keys move along, horizontal by default. */
    readonly orientation?: Exclude<Orientation, "both">;
    /** The StyleX styles applied after the group's styles. */
    readonly style?: style.Styles;
}

/** Read a single or multiple group's value as the list of pressed values. */
function pressedOf(value: string | readonly string[] | undefined): readonly string[] {
    if (value === undefined) {
        return [];
    }

    return typeof value === "string" ? [value] : value;
}

/** The properties of a toggle group's item, the native button's attributes included. */
export interface ToggleGroupItemProperties extends Omit<
    JSX.ButtonHTMLAttributes<HTMLButtonElement>,
    "class" | "style" | "value" | "onClick" | "onFocus"
> {
    /** The value the item stands for. */
    readonly value: string;
    /** The StyleX styles applied after the item's styles. */
    readonly style?: style.Styles;
}

/** Render a group of toggles that arrow keys move between, of which a single group keeps one on. */
export function ToggleGroup(properties: ToggleGroupProperties): JSX.Element {
    // share one group with the items and read the text direction for the arrow keys
    const control = new ToggleGroupControl(properties);
    const locale = useLocale();
    const rest = omit(
        properties,
        "type",
        "value",
        "defaultValue",
        "onValueChange",
        "variant",
        "size",
        "orientation",
        "style",
    );
    const group = merge(DEFAULTS, properties);

    return (
        <ToggleGroupContext value={control}>
            <div
                role={properties.type === "single" ? "radiogroup" : "group"}
                data-slot="toggle-group"
                aria-orientation={group.orientation}
                {...rest}
                onKeyDown={(event) =>
                    moveFocus(
                        event,
                        itemsOf(event.currentTarget, "[data-slot=toggle-group-item]"),
                        group.orientation,
                        directionOf(locale.tag),
                    )
                }
                {...style.attrs(orientations[group.orientation], properties.style)}
            />
        </ToggleGroupContext>
    );
}

/** Render a toggle of the nearest toggle group, exposed as a radio in a single group. */
export function ToggleGroupItem(properties: ToggleGroupItemProperties): JSX.Element {
    // join the group, refusing an item outside one
    const control = useContext(ToggleGroupContext);
    if (control === null) {
        throw new TypeError("a toggle group item needs a toggle group around it");
    }
    control.register(properties.value);

    // report the state as a radio in a single group and as a pressed button otherwise
    const rest = omit(properties, "value", "style");
    const isPressed = (): boolean => control.value().includes(properties.value);
    const isSingle = control.properties.type === "single";

    return (
        <button
            type="button"
            role={isSingle ? "radio" : undefined}
            aria-checked={isSingle ? (isPressed() ? "true" : "false") : undefined}
            aria-pressed={isSingle ? undefined : isPressed() ? "true" : "false"}
            data-slot="toggle-group-item"
            data-state={isPressed() ? "on" : "off"}
            data-value={properties.value}
            tabindex={control.isTabStop(properties.value) ? 0 : -1}
            {...rest}
            onClick={() => control.toggle(properties.value)}
            onFocus={() => control.focus(properties.value)}
            {...style.attrs(
                toggleStyle({
                    variant: control.properties.variant ?? "default",
                    size: control.properties.size ?? "default",
                    pressed: isPressed(),
                }),
                properties.style,
            )}
        />
    );
}
