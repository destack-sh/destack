import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createContext, type JSX, merge, omit, useContext, useLocale } from "@destack/view";
import { type PartAttributes, type Render, useRender } from "../part/index.ts";
import { SELECTION_OPTIONS, Selection, type SelectionProperties } from "../selection/index.ts";
import { ListState } from "../focus/index.ts";
import type { Axis } from "../position/index.ts";
import { toggleVariants, type ToggleSize, type ToggleVariant } from "../toggle/index.ts";

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
    /** The items in document order and the one with the tab stop. */
    readonly list: ListState;

    /** Create the state of a toggle group from its root's properties. */
    constructor(properties: ToggleGroupProperties) {
        // put the tab stop on the focused item, else the first pressed, else the first
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
export type ToggleGroupProperties = Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "onKeyDown"
> & {
    /** The look of every item, default by default. */
    readonly variant?: ToggleVariant;
    /** The height and padding of every item, default by default. */
    readonly size?: ToggleSize;
    /** The direction the arrow keys move along, horizontal by default. */
    readonly orientation?: Axis;
    /** Whether every toggle of the group is unavailable. */
    readonly disabled?: boolean;
    /** The StyleX styles applied after the group's styles. */
    readonly xstyle?: style.Styles;
} & SelectionProperties;

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

/** Read the nearest toggle group, refusing a part outside one. */
export function useToggleGroup(): ToggleGroupControl {
    const control = useContext(ToggleGroupContext);
    if (control === null) {
        throw new TypeError("a toggle group item needs a toggle group around it");
    }

    return control;
}

/** Render a group of toggles that arrow keys move between, of which a single group has one on. */
export function ToggleGroup(properties: ToggleGroupProperties): JSX.Element {
    // share one group with the items and read the text direction for the arrow keys
    const control = new ToggleGroupControl(properties);
    const locale = useLocale();
    const rest = omit(
        properties,
        ...SELECTION_OPTIONS,
        "variant",
        "size",
        "orientation",
        "disabled",
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
                aria-disabled={properties.disabled === true ? "true" : undefined}
                data-disabled={properties.disabled === true ? "" : undefined}
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
    // join the group
    const control = useToggleGroup();
    let element: HTMLElement | undefined;
    const isDisabled = (): boolean =>
        properties.disabled === true || control.properties.disabled === true;
    control.list.add({
        key: properties.value,
        text: () => properties.value,
        isDisabled,
        element: () => element,
    });

    // report the state as a radio in a single group and as a pressed button otherwise
    const rest = omit(properties, "value", "disabled", "xstyle", "style", "render");
    const isPressed = (): boolean => control.selection.isSelected(properties.value);
    const isSingle = (): boolean => control.properties.multiple !== true;
    const state = (): "true" | "false" => (isPressed() ? "true" : "false");
    const part: PartAttributes = merge(
        () =>
            style.attributes(
                [
                    toggleVariants({
                        variant: control.properties.variant ?? "default",
                        size: control.properties.size ?? "default",
                        pressed: isPressed(),
                    }),
                    properties.xstyle,
                ],
                properties.style,
            ),
        {
            get role() {
                return isSingle() ? ("radio" as const) : undefined;
            },
            get "aria-checked"() {
                return isSingle() ? state() : undefined;
            },
            get "aria-pressed"() {
                return isSingle() ? undefined : state();
            },
            "data-slot": "toggle-group-item",
            get "data-pressed"() {
                return isPressed() ? "" : undefined;
            },
            get "data-value"() {
                return properties.value;
            },
            get tabindex() {
                return control.list.focus.isActive(properties.value) ? 0 : -1;
            },
            get disabled() {
                return isDisabled() ? true : undefined;
            },
            ref: (target: HTMLElement) => {
                element = target;
            },
            onClick: () => control.selection.toggle(properties.value),
            onFocus: () => control.list.focus.focusIn(properties.value),
        },
    );

    return useRender({
        render: properties.render,
        defaultTagName: "button",
        part: merge({ type: "button" }, part),
        properties: rest,
    });
}
