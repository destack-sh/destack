import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import {
    color,
    motion,
    radius,
    shadow,
    size,
    space,
    stroke,
    weight,
} from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createContext, type JSX, omit, onCleanup, useContext, useLocale } from "@destack/view";
import { ListState } from "../focus/index.ts";
import type { Direction } from "@destack/locale";
import {
    Menu,
    MenuCheckboxItem,
    MenuContent,
    MenuControl,
    MenuGroup,
    MenuItem,
    MenuLabel,
    MenuRadioGroup,
    MenuRadioItem,
    MenuSeparator,
    MenuShortcut,
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

/** The selector of a menubar's triggers. */
const TRIGGER = "[data-slot=menubar-trigger]";

/** The menubar of the nearest menubar, null outside one. */
const MenubarContext = createContext<MenubarControl | null>(null);

/** The styles of a menubar and its triggers. */
const styles = style.create({
    menubar: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
        height: size[3],
        padding: space[1],
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.background,
        boxShadow: shadow.inset,
    },
    trigger: {
        display: "flex",
        alignItems: "center",
        paddingBlock: space[1],
        paddingInline: space[2],
        borderRadius: radius[2],
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.accent },
            ":focus": color.accent,
        },
        color: {
            default: "inherit",
            ":hover": { default: null, [media.hover]: color.accentForeground },
            ":focus": color.accentForeground,
        },
        fontWeight: weight.medium,
        cursor: "default",
        outlineStyle: "none",
        transitionProperty: "background-color",
        transitionDuration: motion.durationShort,
    },
    open: {
        backgroundColor: color.accent,
        color: color.accentForeground,
    },
});

/** The menus of a menubar in order and the one whose trigger holds the tab stop. */
export class MenubarControl {
    /** The triggers in document order, by their menu's id, which left and right move between. */
    readonly list: ListState;
    /** The menu of each trigger's key. */
    readonly #menus: Map<string, MenuControl>;

    /** Create a menubar without menus. */
    constructor() {
        this.list = new ListState({
            orientation: "horizontal",
            isLooping: true,
            isTypeahead: false,
        });
        this.#menus = new Map();
    }

    /** Add a menu until it unmounts, letting its open menu cross to its neighbours. */
    register(menu: MenuControl): void {
        this.#menus.set(menu.id, menu);
        menu.onCross = (event, direction) => this.cross(menu, event, direction);
        onCleanup(() => this.#menus.delete(menu.id));
    }

    /** Report whether any menu of the bar is open. */
    isOpen(): boolean {
        return [...this.#menus.values()].some((menu) => menu.isOpen());
    }

    /** Close every menu of the bar. */
    closeAll(): void {
        for (const menu of this.#menus.values()) {
            menu.close(false);
        }
    }

    /** Close an open menu and open its neighbour in the arrow key's direction, wrapping around. */
    cross(menu: MenuControl, event: KeyboardEvent, direction: Direction): void {
        // pick the neighbour, mirrored in right-to-left text
        const isForward = (event.key === "ArrowRight") === (direction === "ltr");
        const key = isForward
            ? this.list.delegate.after(menu.id)
            : this.list.delegate.before(menu.id);
        const next = key === undefined ? undefined : this.#menus.get(key);
        if (next === undefined) {
            return;
        }

        // move the open menu and the tab stop with it
        event.preventDefault();
        menu.close(false);
        this.list.focus.focus(next.id);
        next.open("first");
    }
}

/** Find the menubar around an element, refusing elements outside one. */
function useMenubar(): MenubarControl {
    const control = useContext(MenubarContext);
    if (control === null) {
        throw new TypeError("menubar elements need a menubar around them");
    }

    return control;
}

/** Render a bar of menus whose triggers left and right arrow keys move between. */
export function Menubar(properties: MenuElementProperties<HTMLDivElement>): JSX.Element {
    // share one bar with its menus and read the text direction
    const control = new MenubarControl();
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style");

    return (
        <MenubarContext value={control}>
            <div
                role="menubar"
                data-slot="menubar"
                {...rest}
                onKeyDown={(event) => {
                    // move between the triggers, leaving keys from inside a menu to the menu
                    if (event.target instanceof Element && event.target.matches(TRIGGER)) {
                        control.list.focus.move(event, locale.direction);
                    }
                }}
                onFocusOut={(event) => control.list.focus.focusOut(event)}
                {...style.attributes([styles.menubar, properties.xstyle], properties.style)}
            />
        </MenubarContext>
    );
}

/** Hold the open state of one menu of a menubar. */
export function MenubarMenu(properties: MenuRootProperties): JSX.Element {
    // join the bar with a top menu of its own
    const bar = useMenubar();
    const menu = new MenuControl(null, properties);
    bar.register(menu);

    return <Menu control={menu}>{properties.children}</Menu>;
}

/** Render the trigger of a menubar menu, opening it on click, Enter, Space or the down arrow key. */
export function MenubarTrigger(
    properties: Omit<
        MenuElementProperties<HTMLButtonElement>,
        "ref" | "onClick" | "onKeyDown" | "onFocus" | "onPointerEnter"
    >,
): JSX.Element {
    // read the bar and the menu the trigger opens
    const bar = useMenubar();
    const menu = useMenu();
    const rest = omit(properties, "xstyle", "style");
    let element: HTMLElement | undefined;
    bar.list.add({
        key: menu.id,
        text: () => menu.id,
        isDisabled: () => false,
        element: () => element,
    });

    return (
        <button
            type="button"
            id={menu.triggerId}
            role="menuitem"
            aria-haspopup="menu"
            aria-expanded={menu.isOpen() ? "true" : "false"}
            aria-controls={menu.id}
            tabindex={bar.list.focus.isActive(menu.id) ? 0 : -1}
            data-slot="menubar-trigger"
            {...rest}
            ref={(trigger) => {
                element = trigger;
                menu.setTrigger(trigger);
            }}
            onClick={() => (menu.isOpen() ? menu.close(true) : menu.open("first"))}
            onKeyDown={(event) => {
                // open on the down arrow key, focusing the first item
                if (event.key === "ArrowDown") {
                    event.preventDefault();
                    menu.open("first");
                }
            }}
            onFocus={() => bar.list.focus.focusIn(menu.id)}
            onPointerEnter={(event) => {
                // follow the pointer to this menu while another one is open
                if (bar.isOpen() && !menu.isOpen()) {
                    bar.closeAll();
                    event.currentTarget.focus();
                    menu.open("none");
                }
            }}
            {...style.attributes(
                [text.footnote, styles.trigger, menu.isOpen() && styles.open, properties.xstyle],
                properties.style,
            )}
        />
    );
}

/** Render a menubar menu below its trigger. */
export function MenubarContent(properties: MenuContentProperties): JSX.Element {
    return <MenuContent data-slot="menubar-content" {...properties} />;
}

/** Render a group of related items. */
export function MenubarGroup(properties: MenuElementProperties<HTMLDivElement>): JSX.Element {
    return <MenuGroup data-slot="menubar-group" {...properties} />;
}

/** Render a label over a group of items. */
export function MenubarLabel(
    properties: MenuElementProperties<HTMLDivElement> & { readonly inset?: boolean },
): JSX.Element {
    return <MenuLabel data-slot="menubar-label" {...properties} />;
}

/** Render an item that runs its action and closes the menu. */
export function MenubarItem(properties: MenuItemProperties): JSX.Element {
    return <MenuItem data-slot="menubar-item" {...properties} />;
}

/** Render an item that checks and unchecks. */
export function MenubarCheckboxItem(properties: MenuCheckboxItemProperties): JSX.Element {
    return <MenuCheckboxItem data-slot="menubar-checkbox-item" {...properties} />;
}

/** Render a group of radio items that share one selected value. */
export function MenubarRadioGroup(properties: MenuRadioGroupProperties): JSX.Element {
    return <MenuRadioGroup data-slot="menubar-radio-group" {...properties} />;
}

/** Render an item of a radio group. */
export function MenubarRadioItem(properties: MenuRadioItemProperties): JSX.Element {
    return <MenuRadioItem data-slot="menubar-radio-item" {...properties} />;
}

/** Render a line between groups of items. */
export function MenubarSeparator(properties: MenuElementProperties<HTMLDivElement>): JSX.Element {
    return <MenuSeparator data-slot="menubar-separator" {...properties} />;
}

/** Render the keyboard shortcut of an item. */
export function MenubarShortcut(properties: MenuElementProperties<HTMLSpanElement>): JSX.Element {
    return <MenuShortcut data-slot="menubar-shortcut" {...properties} />;
}

/** Hold the open state of a submenu. */
export function MenubarSub(properties: { readonly children?: JSX.Element }): JSX.Element {
    return <Menu>{properties.children}</Menu>;
}

/** Render the item that opens its submenu. */
export function MenubarSubTrigger(
    properties: Omit<MenuItemProperties, "onSelect" | "variant">,
): JSX.Element {
    return <MenuSubTrigger data-slot="menubar-sub-trigger" {...properties} />;
}

/** Render a submenu beside its trigger. */
export function MenubarSubContent(properties: Omit<MenuContentProperties, "side">): JSX.Element {
    return (
        <MenuContent data-slot="menubar-sub-content" side="right" align="start" {...properties} />
    );
}
