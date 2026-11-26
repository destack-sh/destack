import * as style from "@destack/style";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { createContext, type JSX, omit, useContext, useLocale } from "@destack/view";
import { type Render, renderPart } from "../part/index.ts";
import { SELECTION_OPTIONS, Selection, type SelectionProperties } from "../selection/index.ts";
import { ListState } from "../focus/index.ts";
import { Toggle } from "../toggle/index.ts";
import {
    type ComponentProperties,
    type RenderProperties,
    defineComponent,
} from "@destack/view/declare";
import { field } from "@destack/object";
import { t } from "@destack/locale";
import { readChoices, useFieldControl, useFieldValue } from "../field/control.ts";

/** The group of the nearest toggle group, null outside one. */
const ToggleGroupContext = createContext<ToggleGroupControl | null>(null);

/** The layout of a toggle group in each orientation. */
const orientations = style.create({
    horizontal: { display: "flex", alignItems: "center", gap: space[1] },
    vertical: { display: "flex", flexDirection: "column", gap: space[1] },
});

/** The track a segmented group lays its items on. */
const variants = style.create({
    segmented: {
        gap: 0,
        padding: stroke.ring,
        borderRadius: radius[3],
        backgroundColor: color.muted,
    },
});

/** The pressed values and focus of a toggle group, which its items share. */
export class ToggleGroupControl {
    /** The properties of the group root, read for its controlled state and look. */
    readonly properties: RenderProperties<ToggleGroupAttributes, typeof TOGGLE_GROUP_PROPERTIES>;
    /** The pressed values, controlled or the group's own. */
    readonly selection: Selection;
    /** The items in document order and the one with the tab stop. */
    readonly list: ListState;

    /** Create the state of a toggle group from its root's properties. */
    constructor(
        properties: RenderProperties<ToggleGroupAttributes, typeof TOGGLE_GROUP_PROPERTIES>,
    ) {
        // put the tab stop on the focused item, else the first pressed, else the first
        this.properties = properties;
        this.selection = new Selection(properties);
        this.list = new ListState({
            get orientation() {
                return properties.orientation;
            },
            isLooping: true,
            isTypeahead: false,
            initial: (items) => items.keys().find((key) => this.selection.isSelected(key)),
        });
    }
}

/** The properties of a toggle group, the native element's attributes included. */
export type ToggleGroupAttributes = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> & {
    /** The StyleX styles applied after the group's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
} & SelectionProperties;

/** The properties of a toggle group's item, the native button's attributes included. */
export interface ToggleGroupItemProperties extends Omit<
    JSX.ButtonHTMLAttributes<HTMLButtonElement>,
    "class" | "value"
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

/** The properties people change in a toggle group's controls and inspector. */
const TOGGLE_GROUP_PROPERTIES = {
    ...Toggle.properties,
    orientation: field
        .enum({
            horizontal: { title: t.context("option")`Horizontal` },
            vertical: { title: t.context("option")`Vertical` },
        })
        .labeled(t.context("label")`Orientation`)
        .described(t`Whether the toggles lie in a row or a column.`)
        .default("horizontal"),
};

/** Render a group of toggles that arrow keys move between, of which a single group has one on. */
export const ToggleGroup = defineComponent({
    name: "toggleGroup",
    properties: TOGGLE_GROUP_PROPERTIES,
    render(
        given: RenderProperties<ToggleGroupAttributes, typeof TOGGLE_GROUP_PROPERTIES>,
    ): JSX.Element {
        // edit the value of the field's form
        const properties = useFieldValue(given, {
            value: "value",
            change: "onValueChange",
            read: readChoices,
            settles: true,
        });

        // label the group by the nearest field's label, as a field labels a group of controls
        const fieldControl = useFieldControl();
        fieldControl?.group();

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
        );

        return (
            <ToggleGroupContext value={control}>
                {renderPart(
                    "div",
                    "toggle-group",
                    rest,
                    () => [
                        orientations[properties.orientation],
                        properties.variant === "segmented" && variants.segmented,
                    ],
                    {
                        get id() {
                            return fieldControl?.groupAttributes().id;
                        },
                        get "aria-labelledby"() {
                            return fieldControl?.groupAttributes()["aria-labelledby"];
                        },
                        get "aria-describedby"() {
                            return fieldControl?.groupAttributes()["aria-describedby"];
                        },
                        get role() {
                            return properties.multiple === true ? "group" : "radiogroup";
                        },
                        get "aria-orientation"() {
                            return properties.orientation;
                        },
                        get "aria-disabled"() {
                            return properties.disabled ? "true" : undefined;
                        },
                        get "data-disabled"() {
                            return properties.disabled ? "" : undefined;
                        },
                        onKeyDown: (event) => control.list.focus.move(event, locale.direction),
                        onFocusOut: (event) => control.list.focus.focusOut(event),
                    },
                )}
            </ToggleGroupContext>
        );
    },
});

/** Render a toggle of the nearest toggle group, exposed as a radio in a single group. */
export function ToggleGroupItem(properties: ToggleGroupItemProperties): JSX.Element {
    // join the group
    const control = useToggleGroup();
    let element: HTMLElement | undefined;
    const isDisabled = (): boolean => properties.disabled === true || control.properties.disabled;
    control.list.add({
        key: properties.value,
        text: () => properties.value,
        isDisabled,
        element: () => element,
    });

    // report the state as a radio in a single group and as a pressed button otherwise
    const rest = omit(properties, "value", "disabled");
    const isPressed = (): boolean => control.selection.isSelected(properties.value);
    const isSingle = (): boolean => control.properties.multiple !== true;
    const state = (): "true" | "false" => (isPressed() ? "true" : "false");

    return renderPart(
        "button",
        "toggle-group-item",
        rest,
        () =>
            Toggle.styles({
                variant: control.properties.variant,
                size: control.properties.size,
                pressed: isPressed(),
            }),
        {
            type: "button" as const,
            get role() {
                return isSingle() ? ("radio" as const) : undefined;
            },
            get "aria-checked"() {
                return isSingle() ? state() : undefined;
            },
            get "aria-pressed"() {
                return isSingle() ? undefined : state();
            },
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
}

/** The properties of a toggle group, its native attributes included. */
export type ToggleGroupProperties = ComponentProperties<typeof ToggleGroup>;
