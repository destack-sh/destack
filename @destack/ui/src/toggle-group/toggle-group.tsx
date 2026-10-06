import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { useLocale } from "@destack/locale/solid";
import {
    type Accessor,
    createContext,
    createSignal,
    type JSX,
    merge,
    omit,
    onCleanup,
    type Setter,
    useContext,
} from "@destack/view";
import { type PartAttributes, type Render, rendered } from "../part/index.ts";
import { type Choice, createChoice, toggled } from "../choice/index.ts";
import { itemsOf, moveFocus, type Orientation } from "../focus/index.ts";
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

/** The pressed values and focus of a toggle group, which its items share. */
export class ToggleGroupControl {
    /** The properties of the group root, read for its controlled state and look. */
    readonly properties: ToggleGroupProperties;
    /** The pressed values, controlled or the group's own. */
    readonly value: Accessor<readonly string[]>;
    /** The values of the items in document order. */
    readonly values: Accessor<readonly string[]>;
    /** The value of the item the focus last rested on. */
    readonly focused: Accessor<string | undefined>;
    /** Replace the pressed values and tell the change handler. */
    readonly #setValue: (value: readonly string[]) => void;
    /** Replace the values of the items. */
    readonly #setValues: Setter<readonly string[]>;
    /** Replace the value of the focused item. */
    readonly #setFocused: Setter<string | undefined>;

    /** Create the state of a toggle group from its root's properties. */
    constructor(properties: ToggleGroupProperties) {
        // start from the default value with no items and no focus
        const [value, setValue] = createChoice(properties);
        const [values, setValues] = createSignal<readonly string[]>([], { ownedWrite: true });
        const [focused, setFocused] = createSignal<string | undefined>(undefined);
        this.properties = properties;
        this.value = value;
        this.values = values;
        this.focused = focused;
        this.#setValue = setValue;
        this.#setValues = setValues;
        this.#setFocused = setFocused;
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
        this.#setValue(toggled(this.properties, this.value(), value));
    }

    /** Report whether an item takes the tab stop: the focused one, else the first pressed, else the first. */
    isTabStop(value: string): boolean {
        const values = this.values();
        const stop =
            this.focused() ?? values.find((entry) => this.value().includes(entry)) ?? values[0];

        return stop === value;
    }
}

/** The properties of a toggle group, the native element's attributes included. */
export type ToggleGroupProperties = ToggleGroupLook & Choice;

/** The look and layout of a toggle group, the native element's attributes included. */
export interface ToggleGroupLook extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "onKeyDown"
> {
    /** The look of every item, default by default. */
    readonly variant?: ToggleVariant;
    /** The height and padding of every item, default by default. */
    readonly size?: ToggleSize;
    /** The direction the arrow keys move along, horizontal by default. */
    readonly orientation?: Exclude<Orientation, "both">;
    /** The StyleX styles applied after the group's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of a toggle group's item, the native button's attributes included. */
export interface ToggleGroupItemProperties extends Omit<
    JSX.ButtonHTMLAttributes<HTMLButtonElement>,
    "class" | "value" | "onClick" | "onFocus"
> {
    /** The value the item stands for. */
    readonly value: string;
    /** The StyleX styles applied after the item's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with the item's attributes, the native button by default. */
    readonly render?: Render;
}

/** Render a group of toggles that arrow keys move between, of which a single group keeps one on. */
export function ToggleGroup(properties: ToggleGroupProperties): JSX.Element {
    // share one group with the items and read the text direction for the arrow keys
    const control = new ToggleGroupControl(properties);
    const locale = useLocale();
    const rest = omit(
        properties,
        "multiple",
        "value",
        "defaultValue",
        "onValueChange",
        "variant",
        "size",
        "orientation",
        "xstyle",
        "style",
    );
    const group = merge(DEFAULTS, properties);

    return (
        <ToggleGroupContext value={control}>
            <div
                role={properties.multiple === true ? "group" : "radiogroup"}
                data-slot="toggle-group"
                aria-orientation={group.orientation}
                {...rest}
                onKeyDown={(event) =>
                    moveFocus(
                        event,
                        itemsOf(event.currentTarget, "[data-slot=toggle-group-item]"),
                        group.orientation,
                        locale.direction,
                    )
                }
                {...style.attributes(
                    [orientations[group.orientation], properties.xstyle],
                    properties.style,
                )}
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
    const rest = omit(properties, "value", "xstyle", "style", "render");
    const isPressed = (): boolean => control.value().includes(properties.value);
    const isSingle = control.properties.multiple !== true;
    const state = (): string => (isPressed() ? "true" : "false");
    const part: PartAttributes = merge(
        {
            role: isSingle ? ("radio" as const) : undefined,
            get "aria-checked"() {
                return isSingle ? state() : undefined;
            },
            get "aria-pressed"() {
                return isSingle ? undefined : state();
            },
            "data-slot": "toggle-group-item",
            get "data-state"() {
                return isPressed() ? "on" : "off";
            },
            get "data-value"() {
                return properties.value;
            },
            get tabindex() {
                return control.isTabStop(properties.value) ? 0 : -1;
            },
            onClick: () => control.toggle(properties.value),
            onFocus: () => control.focus(properties.value),
        },
        () =>
            style.attributes(
                [
                    toggleStyle({
                        variant: control.properties.variant ?? "default",
                        size: control.properties.size ?? "default",
                        pressed: isPressed(),
                    }),
                    properties.xstyle,
                ],
                properties.style,
            ),
    );

    return rendered(properties.render, part, rest, () => (
        <button type="button" {...part} {...rest} />
    ));
}
