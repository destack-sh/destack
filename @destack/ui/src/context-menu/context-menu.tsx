import { type JSX } from "@destack/view";
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
import { renderPart } from "../part/index.ts";

/** Hold the open state of a menu that a right click or the context menu key opens. */
export function ContextMenu(properties: MenuRootProperties): JSX.Element {
    return <Menu control={new MenuControl(null, properties)}>{properties.children}</Menu>;
}

/** Render the area whose context menu event opens the menu at the pointer instead of the browser's. */
export function ContextMenuTrigger(
    properties: Omit<MenuElementProperties<HTMLDivElement>, "ref" | "onContextMenu">,
): JSX.Element {
    const control = useMenu();

    return renderPart("div", "context-menu-trigger", properties, null, {
        ref: (element) => control.setTrigger(element),
        onContextMenu: (event) => {
            // open at the pointer and focus the first item
            event.preventDefault();
            control.open("first", { x: event.clientX, y: event.clientY });
        },
    });
}

/** Render the menu at the point it opened at. */
export function ContextMenuContent(
    properties: Omit<MenuContentProperties, "side" | "align">,
): JSX.Element {
    return <MenuContent data-slot="context-menu-content" {...properties} />;
}

/** Render a group of related items. */
export function ContextMenuGroup(properties: MenuElementProperties<HTMLDivElement>): JSX.Element {
    return <MenuGroup data-slot="context-menu-group" {...properties} />;
}

/** Render a label over a group of items. */
export function ContextMenuLabel(
    properties: MenuElementProperties<HTMLDivElement> & { readonly inset?: boolean },
): JSX.Element {
    return <MenuLabel data-slot="context-menu-label" {...properties} />;
}

/** Render an item that runs its action and closes the menu. */
export function ContextMenuItem(properties: MenuItemProperties): JSX.Element {
    return <MenuItem data-slot="context-menu-item" {...properties} />;
}

/** Render an item that checks and unchecks. */
export function ContextMenuCheckboxItem(properties: MenuCheckboxItemProperties): JSX.Element {
    return <MenuCheckboxItem data-slot="context-menu-checkbox-item" {...properties} />;
}

/** Render a group of radio items that share one selected value. */
export function ContextMenuRadioGroup(properties: MenuRadioGroupProperties): JSX.Element {
    return <MenuRadioGroup data-slot="context-menu-radio-group" {...properties} />;
}

/** Render an item of a radio group. */
export function ContextMenuRadioItem(properties: MenuRadioItemProperties): JSX.Element {
    return <MenuRadioItem data-slot="context-menu-radio-item" {...properties} />;
}

/** Render a line between groups of items. */
export function ContextMenuSeparator(
    properties: MenuElementProperties<HTMLDivElement>,
): JSX.Element {
    return <MenuSeparator data-slot="context-menu-separator" {...properties} />;
}

/** Render the keyboard shortcut of an item. */
export function ContextMenuShortcut(
    properties: MenuElementProperties<HTMLSpanElement>,
): JSX.Element {
    return <MenuShortcut data-slot="context-menu-shortcut" {...properties} />;
}

/** Hold the open state of a submenu. */
export function ContextMenuSub(properties: { readonly children?: JSX.Element }): JSX.Element {
    return <Menu>{properties.children}</Menu>;
}

/** Render the item that opens its submenu. */
export function ContextMenuSubTrigger(
    properties: Omit<MenuItemProperties, "onSelect" | "variant">,
): JSX.Element {
    return <MenuSubTrigger data-slot="context-menu-sub-trigger" {...properties} />;
}

/** Render a submenu beside its trigger. */
export function ContextMenuSubContent(
    properties: Omit<MenuContentProperties, "side">,
): JSX.Element {
    return (
        <MenuContent
            data-slot="context-menu-sub-content"
            side="right"
            align="start"
            {...properties}
        />
    );
}
