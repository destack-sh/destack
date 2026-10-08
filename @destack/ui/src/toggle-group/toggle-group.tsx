import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createContext, type JSX, merge, omit, useContext, useLocale } from "@destack/view";
import { type PartAttributes, type Render, rendered } from "../part/index.ts";
import { Selection, type SelectionProperties } from "../selection/index.ts";
import { ListState, type Orientation } from "../focus/index.ts";
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
    readonly selection: Selection;
    /** The items in document order and the one holding the tab stop. */
    readonly list: ListState;

    /** Create the state of a toggle group from its root's properties. */
    constructor(properties: ToggleGroupProperties) {
        // hold the tab stop on the focused item, else the first pressed, else the first
        const group = merge(DEFAULTS, properties);
        this.properties = properties;
        this.selection = new Selection(properties);
        this.list = new ListState({
            get orientation() {
                return group.orientation;
            },
            isLooping: true,
            isTypeahead: false,
            initial: (items) => items.keys().find((key) => this.selection.isSelected(key)),
        });
    }
}

/** The properties of a toggle group, the native element's attributes included. */
export type ToggleGroupProperties = ToggleGroupLook & SelectionProperties;

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
                onKeyDown={(event) => control.list.focus.move(event, locale.direction)}
                onFocusOut={(event) => control.list.focus.focusOut(event)}
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
    let element: HTMLElement | undefined;
    control.list.add({
        key: properties.value,
        text: () => properties.value,
        isDisabled: () => properties.disabled === true,
        element: () => element,
    });

    // report the state as a radio in a single group and as a pressed button otherwise
    const rest = omit(properties, "value", "xstyle", "style", "render");
    const isPressed = (): boolean => control.selection.isSelected(properties.value);
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
                return control.list.focus.isActive(properties.value) ? 0 : -1;
            },
            ref: (target: HTMLElement) => {
                element = target;
            },
            onClick: () => control.selection.toggle(properties.value),
            onFocus: () => control.list.focus.focusIn(properties.value),
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
