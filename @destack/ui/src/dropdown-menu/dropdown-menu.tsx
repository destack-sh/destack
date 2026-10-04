import type { JSX } from "@solidjs/web";
import { Button, type ButtonProperties } from "../button/index.ts";
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

/** The properties of a dropdown menu's trigger, a button's properties included. */
export type DropdownMenuTriggerProperties = Omit<ButtonProperties, "ref" | "onClick" | "onKeyDown">;

/** Hold the open state of a menu that a button opens. */
export function DropdownMenu(properties: MenuRootProperties): JSX.Element {
    return <Menu control={new MenuControl(null, properties)}>{properties.children}</Menu>;
}

/** Render the button that opens its menu on click, Enter, Space and the up and down arrow keys. */
export function DropdownMenuTrigger(properties: DropdownMenuTriggerProperties): JSX.Element {
    const control = useMenu();

    return (
        <Button
            id={control.triggerId}
            data-slot="dropdown-menu-trigger"
            aria-haspopup="menu"
            aria-expanded={control.isOpen() ? "true" : "false"}
            aria-controls={control.id}
            {...properties}
            ref={(element) => control.setTrigger(element)}
            onClick={() => (control.isOpen() ? control.close(true) : control.open("first"))}
            onKeyDown={(event) => {
                // open on the arrow keys, focusing the first or the last item
                if (event.key === "ArrowDown" || event.key === "ArrowUp") {
                    event.preventDefault();
                    control.open(event.key === "ArrowDown" ? "first" : "last");
                }
            }}
        />
    );
}

/** Render the menu beside its trigger. */
export function DropdownMenuContent(properties: MenuContentProperties): JSX.Element {
    return <MenuContent data-slot="dropdown-menu-content" {...properties} />;
}

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

/** Render an item that runs its action and closes the menu. */
export function DropdownMenuItem(properties: MenuItemProperties): JSX.Element {
    return <MenuItem data-slot="dropdown-menu-item" {...properties} />;
}

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
export function DropdownMenuShortcut(
    properties: MenuElementProperties<HTMLSpanElement>,
): JSX.Element {
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
