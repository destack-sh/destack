import { type JSX, omit, onCleanup, untrack } from "@destack/view";
import { controlOf, OpenChange, type TriggerProperties } from "../layer/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    Menu,
    MenuCheckboxItem,
    MenuContent,
    MenuGroup,
    MenuItem,
    MenuLabel,
    MenuRadioGroup,
    MenuRadioItem,
    MenuSeparator,
    MenuShortcut,
    type MenuShortcutProperties,
    MenuControl,
    MenuSubTrigger,
    useMenu,
    type MenuCheckboxItemProperties,
    type MenuContentProperties,
    type MenuElementProperties,
    type MenuItemProperties,
    type MenuRadioGroupProperties,
    type MenuRadioItemProperties,
    type MenuRootProperties,
} from "../menu/index.ts";
import { type RenderProperties, defineComponent } from "@destack/view/declare";
import { t } from "@destack/locale";
import { field } from "@destack/object";

/** The properties of a dropdown menu's trigger, a native button's attributes included. */
export type DropdownMenuTriggerProperties<Payload = unknown> = ElementPartProperties &
    Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "class" | "style" | "ref"> &
    TriggerProperties<Payload, MenuControl<Payload>> & {
        /** Whether the up and down arrows open the menu, as they do outside a grid's cells. */
        readonly isOpenedByArrows?: boolean;
    };

/** Hold the open state of a menu that a button opens. */
export function DropdownMenu<Payload = unknown>(
    properties: MenuRootProperties<Payload>,
): JSX.Element {
    const control = controlOf(properties, () => new MenuControl<Payload>(null, properties));

    return <Menu control={control}>{properties.children}</Menu>;
}

/** Render the element that opens its menu on click, Enter, Space and the arrows. */
export function DropdownMenuTrigger<Payload = unknown>(
    properties: DropdownMenuTriggerProperties<Payload>,
): JSX.Element {
    // open its root's menu, or the one it is given from elsewhere
    const given = untrack(() => properties.control);
    const control = given ?? useMenu();
    const id = control.state.triggerId();
    let element: HTMLElement | undefined;
    const triggerOf = (event: Event & { readonly currentTarget: HTMLElement }) => ({
        element: event.currentTarget,
        payload: properties.payload,
    });
    onCleanup(() => given?.drop(element));
    const rest = omit(properties, "control", "payload", "isOpenedByArrows");

    return renderPart("button", "dropdown-menu-trigger", rest, [], {
        type: "button",
        id,
        "aria-haspopup": "menu",
        get "aria-expanded"() {
            return control.state.isFrom(element) ? "true" : "false";
        },
        "aria-controls": control.id,
        ref: (target: HTMLElement) => {
            element = target;
            if (given === undefined) {
                control.setTrigger(target);
            }
        },
        onPointerDown: (event) => control.release.press(event, triggerOf(event)),
        onMouseDown: (event) => {
            // leave the focus to the menu a mouse opens
            if (control.isOpen()) {
                event.preventDefault();
            }
        },
        onClick: (event) => control.release.click(event, triggerOf(event)),
        onKeyDown: (event) => {
            // open on the arrow keys where they open it, focusing the first or the last item
            const isArrow = event.key === "ArrowDown" || event.key === "ArrowUp";
            if (properties.isOpenedByArrows !== false && isArrow) {
                event.preventDefault();
                const focus = event.key === "ArrowDown" ? "first" : "last";
                control.open(focus, new OpenChange("trigger-press", event, triggerOf(event)));
            }
        },
    });
}

/** The properties people change in a dropdown menu's content's controls and inspector. */
const DROPDOWN_MENU_CONTENT_PROPERTIES = {
    side: field
        .enum({
            top: { title: t.context("option")`Top` },
            right: { title: t.context("option")`Right` },
            bottom: { title: t.context("option")`Bottom` },
            left: { title: t.context("option")`Left` },
        })
        .labeled(t.context("label")`Side`)
        .described(t`The side of its trigger the menu opens on, where it fits.`)
        .default("bottom"),
    align: field
        .enum({
            start: { title: t.context("option")`Start` },
            center: { title: t.context("option")`Center` },
            end: { title: t.context("option")`End` },
        })
        .labeled(t.context("label")`Align`)
        .described(t`Where along its trigger the menu lines up.`)
        .default("start"),
};

/** Render the menu beside its trigger. */
export const DropdownMenuContent = defineComponent({
    name: "dropdownMenuContent",
    properties: DROPDOWN_MENU_CONTENT_PROPERTIES,
    render(
        properties: RenderProperties<
            MenuContentProperties,
            typeof DROPDOWN_MENU_CONTENT_PROPERTIES
        >,
    ): JSX.Element {
        return <MenuContent data-slot="dropdown-menu-content" {...properties} />;
    },
});

/** Render a group of related items. */
export function DropdownMenuGroup(properties: MenuElementProperties<HTMLDivElement>): JSX.Element {
    return <MenuGroup data-slot="dropdown-menu-group" {...properties} />;
}

/** Render a label over a group of items. */
export function DropdownMenuLabel(
    properties: MenuElementProperties<HTMLDivElement> & { readonly inset?: boolean },
): JSX.Element {
    return <MenuLabel data-slot="dropdown-menu-label" {...properties} />;
}

/** The properties people change in a dropdown menu item's controls and inspector. */
const DROPDOWN_MENU_ITEM_PROPERTIES = {
    variant: field
        .enum({
            default: { title: t.context("option")`Default` },
            destructive: {
                title: t.context("option")`Destructive`,
                description: t`An item that deletes or cannot be undone.`,
            },
        })
        .labeled(t.context("label")`Variant`)
        .described(t`Whether the item marks a destructive action.`)
        .default("default"),
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses the item where its action cannot run now.`)
        .default(false),
};

/** Render an item that runs its action and closes the menu. */
export const DropdownMenuItem = defineComponent({
    name: "dropdownMenuItem",
    properties: DROPDOWN_MENU_ITEM_PROPERTIES,
    render(
        properties: RenderProperties<MenuItemProperties, typeof DROPDOWN_MENU_ITEM_PROPERTIES>,
    ): JSX.Element {
        return <MenuItem data-slot="dropdown-menu-item" {...properties} />;
    },
});

/** Render an item that checks and unchecks. */
export function DropdownMenuCheckboxItem(properties: MenuCheckboxItemProperties): JSX.Element {
    return <MenuCheckboxItem data-slot="dropdown-menu-checkbox-item" {...properties} />;
}

/** Render a group of radio items that share one selected value. */
export function DropdownMenuRadioGroup(properties: MenuRadioGroupProperties): JSX.Element {
    return <MenuRadioGroup data-slot="dropdown-menu-radio-group" {...properties} />;
}

/** Render an item of a radio group. */
export function DropdownMenuRadioItem(properties: MenuRadioItemProperties): JSX.Element {
    return <MenuRadioItem data-slot="dropdown-menu-radio-item" {...properties} />;
}

/** Render a line between groups of items. */
export function DropdownMenuSeparator(
    properties: MenuElementProperties<HTMLDivElement>,
): JSX.Element {
    return <MenuSeparator data-slot="dropdown-menu-separator" {...properties} />;
}

/** Render the keyboard shortcut of an item. */
export function DropdownMenuShortcut(properties: MenuShortcutProperties): JSX.Element {
    return <MenuShortcut data-slot="dropdown-menu-shortcut" {...properties} />;
}

/** Hold the open state of a submenu. */
export function DropdownMenuSub(properties: { readonly children?: JSX.Element }): JSX.Element {
    return <Menu>{properties.children}</Menu>;
}

/** Render the item that opens its submenu. */
export function DropdownMenuSubTrigger(
    properties: Omit<MenuItemProperties, "onSelect" | "variant">,
): JSX.Element {
    return <MenuSubTrigger data-slot="dropdown-menu-sub-trigger" {...properties} />;
}

/** Render a submenu beside its trigger. */
export function DropdownMenuSubContent(
    properties: Omit<MenuContentProperties, "side">,
): JSX.Element {
    return (
        <MenuContent
            data-slot="dropdown-menu-sub-content"
            side="right"
            align="start"
            {...properties}
        />
    );
}
