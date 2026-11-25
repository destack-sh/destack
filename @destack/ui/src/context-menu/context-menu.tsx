import * as style from "@destack/style";
import { type JSX, omit, onCleanup, untrack } from "@destack/view";
import { Timeout } from "@destack/view/primitives/timer";
import { MenuActionList } from "../action/action-menu.tsx";
import { focusableAround } from "../focus/index.ts";
import { controlOf, OpenChange, type TriggerProperties } from "../layer/index.ts";
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
import { contentOr, renderPart } from "../part/index.ts";

/** The time a finger stays before it opens the menu, in milliseconds, as iOS and Android wait. */
const LONG_PRESS = 500;

/** The styles of the area a context menu opens over. */
const styles = style.create({
    trigger: {
        // leave a long press to the menu on iOS
        WebkitTouchCallout: "none",
    },
});

/** Hold the open state of a menu that a right click or the context menu key opens. */
export function ContextMenu<Payload = unknown>(
    properties: MenuRootProperties<Payload>,
): JSX.Element {
    const control = controlOf(properties, () => new MenuControl<Payload>(null, properties));

    return <Menu control={control}>{properties.children}</Menu>;
}

/** Render the area whose context menu event opens the menu at the pointer. */
export function ContextMenuTrigger<Payload = unknown>(
    properties: TriggerProperties<Payload, MenuControl<Payload>> &
        Omit<
            MenuElementProperties<HTMLDivElement>,
            | "ref"
            | "onContextMenu"
            | "onPointerDown"
            | "onPointerMove"
            | "onPointerUp"
            | "onPointerCancel"
        >,
): JSX.Element {
    // open its root's menu, or the one it is given from elsewhere
    const given = untrack(() => properties.control);
    const control = given ?? useMenu();
    const rest = omit(properties, "control", "payload");
    const timer = new Timeout();
    let element: HTMLElement | undefined;
    onCleanup(() => given?.drop(element));
    const change = (event: Event & { readonly currentTarget: HTMLElement }) =>
        new OpenChange("trigger-press", event, {
            element: focusableAround(event.target, event.currentTarget),
            payload: properties.payload,
        });

    return renderPart("div", "context-menu-trigger", rest, styles.trigger, {
        ref: (target) => {
            element = target;
            if (given === undefined) {
                control.setTrigger(target);
            }
        },
        onContextMenu: (event) => {
            // open at the pointer, wait for one still down, and focus the first item
            event.preventDefault();
            const point = { x: event.clientX, y: event.clientY };
            control.open("first", change(event), point);
            if (event.buttons !== 0) {
                control.release.await(event);
            }
        },
        onPointerDown: (event) => {
            // open under a finger that stays
            if (event.pointerType === "touch") {
                const point = { x: event.clientX, y: event.clientY };
                const pressed = change(event);
                timer.set(() => {
                    control.open("first", pressed, point);
                    control.release.await(event);
                }, LONG_PRESS);
            }
        },
        onPointerMove: (event) => {
            // leave a finger that scrolls alone
            if (event.pointerType === "touch") {
                timer.clear();
            }
        },
        onPointerUp: () => timer.clear(),
        onPointerCancel: () => timer.clear(),
    });
}

/** Render the menu at the point it opened at, the actions on what it is on by default. */
export function ContextMenuContent(
    properties: Omit<MenuContentProperties, "side" | "align">,
): JSX.Element {
    return (
        <MenuContent data-slot="context-menu-content" {...properties}>
            {contentOr(properties, () => (
                <MenuActionList />
            ))}
        </MenuContent>
    );
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
