import { Icon } from "@destack/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import * as style from "@destack/style";
import { color, radius, shadow, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createEffect,
    createSignal,
    createUniqueId,
    type JSX,
    merge,
    omit,
    type Setter,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import {
    type ElementPartProperties,
    type PartAttributes,
    type PartEvent,
    type Render,
    rendered,
    renderPart,
} from "../part/index.ts";
import type { Direction } from "@destack/locale";
import { ListState } from "../focus/index.ts";
import { type Align, type Point, Position, type Side } from "../position/index.ts";
import { TopLayer } from "../layer/index.ts";

/** The side and alignment of a menu that sets neither. */
const DEFAULTS: Required<Pick<MenuContentProperties, "side" | "align">> = {
    side: "bottom",
    align: "start",
};

/** The menu of the nearest menu, null outside one. */
const MenuContext = createContext<MenuControl | null>(null);

/** The selected value of the nearest radio group of a menu, null outside one. */
const MenuRadioContext = createContext<MenuRadioControl | null>(null);

/** The styles of a menu and its elements. */
const styles = style.create({
    content: {
        minWidth: `calc(8 * ${space[4]})`,
        maxHeight: "100vh",
        overflowY: "auto",
        inset: "auto",
        margin: space[1],
        padding: space[1],
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.popover,
        color: color.popoverForeground,
        boxShadow: shadow.overlay,
    },
    item: {
        position: "relative",
        display: "flex",
        alignItems: "center",
        gap: space[2],
        paddingBlock: space[1],
        paddingInline: space[2],
        borderRadius: radius[2],
        outlineStyle: "none",
        backgroundColor: { default: "transparent", ":focus": color.accent },
        color: { default: "inherit", ":focus": color.accentForeground },
        cursor: "default",
        userSelect: "none",
    },
    destructive: {
        color: { default: color.destructive, ":focus": color.destructive },
        backgroundColor: {
            default: "transparent",
            ":focus": `color-mix(in oklab, ${color.destructive} 10%, transparent)`,
        },
    },
    disabled: {
        opacity: 0.5,
        pointerEvents: "none",
    },
    inset: {
        paddingInlineStart: space[6],
    },
    indicator: {
        position: "absolute",
        insetInlineStart: space[2],
        display: "inline-flex",
    },
    label: {
        paddingBlock: space[1],
        paddingInline: space[2],
        fontWeight: weight.medium,
    },
    separator: {
        height: 0,
        marginBlock: space[1],
        marginInline: `calc(-1 * ${space[1]})`,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        borderTopColor: color.border,
    },
    shortcut: {
        marginInlineStart: "auto",
        color: color.mutedForeground,
        letterSpacing: "0.1em",
    },
    chevron: {
        marginInlineStart: "auto",
        display: "inline-flex",
    },
});

/** The item a newly opened menu focuses. */
export type MenuFocus = "first" | "last" | "none";

/** The open state, elements and focus of a menu or submenu, which its trigger, items and submenus share. */
export class MenuControl {
    /** The id of the menu element. */
    readonly id: string;
    /** The id of the trigger that opens the menu and names it, unless it opens at a point. */
    readonly triggerId: string;
    /** The menu this one is a submenu of, null for a top menu. */
    readonly parent: MenuControl | null;
    /** Whether the menu is open. */
    readonly isOpen: Accessor<boolean>;
    /** The point a context menu opens at, undefined for a menu that opens beside its trigger. */
    readonly point: Accessor<Point | undefined>;
    /** The menu's own items in document order, which the arrow keys, Home, End and typed letters move through. */
    readonly list: ListState;
    /** Handle a horizontal arrow key a top menu leaves unused, such as a menubar moving to the next menu. */
    onCross: ((event: KeyboardEvent, direction: Direction) => void) | undefined;
    /** The element that opens the menu and anchors it. */
    #trigger: HTMLElement | undefined;
    /** The menu element. */
    #content: HTMLElement | undefined;
    /** The item to focus once the menu shows. */
    #focus: MenuFocus;
    /** Whether the trigger takes the focus back once the menu hides. */
    #isReturningFocus: boolean;
    /** Stop placing the shown menu beside its trigger. */
    #unplace: () => void;
    /** The open submenus. */
    readonly #children: Set<MenuControl>;
    /** Replace whether the menu is open and tell the change handler. */
    readonly #setOpen: (isOpen: boolean) => void;
    /** Replace the point a context menu opens at. */
    readonly #setPoint: Setter<Point | undefined>;

    /** Create a closed menu, a submenu when it has a parent. */
    constructor(parent: MenuControl | null, properties: MenuRootProperties = {}) {
        // follow the controlled state, else the menu's own, without elements yet
        const [isOpen, setOpen] = createControllableSignal({
            isControlled: () => properties.open !== undefined,
            value: () => properties.open === true,
            defaultValue: properties.defaultOpen === true,
            onChange: (isNext) => properties.onOpenChange?.(isNext),
        });
        const [point, setPoint] = createSignal<Point | undefined>(undefined);
        this.id = createUniqueId();
        this.triggerId = `${this.id}-trigger`;
        this.parent = parent;
        this.isOpen = isOpen;
        this.point = point;
        this.list = new ListState({ orientation: "vertical", isLooping: true, isTypeahead: true });
        this.onCross = undefined;
        this.#trigger = undefined;
        this.#content = undefined;
        this.#focus = "first";
        this.#isReturningFocus = false;
        this.#unplace = () => undefined;
        this.#children = new Set();
        this.#setOpen = setOpen;
        this.#setPoint = setPoint;
    }

    /** Set the element that opens the menu and anchors it. */
    setTrigger(element: HTMLElement): void {
        this.#trigger = element;
    }

    /** Set the menu element and show or hide it as the menu opens and closes. */
    setContent(element: HTMLElement): void {
        this.#content = element;
    }

    /** Show or hide the menu element for an open state, focusing the item asked for when it shows. */
    sync(isOpen: boolean): void {
        // wait for the menu element to mount
        const content = this.#content;
        if (content === undefined) {
            return;
        }

        // show it beside its trigger, or at its point, and focus the item asked for
        if (isOpen) {
            const trigger = this.#trigger;
            const isBeside = trigger !== undefined && this.point() === undefined;
            content.showPopover(isBeside ? { source: trigger } : undefined);
            this.#unplace = isBeside ? Position.place(content, trigger) : () => undefined;
            untrack(() => this.#focusItem(content));
        } else {
            this.#unplace();
            content.hidePopover();
            if (this.#isReturningFocus) {
                this.#trigger?.focus();
            }
        }
        this.#isReturningFocus = false;
    }

    /** Focus the item a newly shown menu asks for, or the menu itself when it has no items. */
    #focusItem(content: HTMLElement): void {
        // leave the focus alone, else take the first or last available item
        if (this.#focus === "none") {
            return;
        }
        const delegate = this.list.delegate;
        const target = this.#focus === "first" ? delegate.first() : delegate.last();

        // focus the item, or the menu itself without one
        if (target === undefined) {
            content.focus();
        } else {
            this.list.focus.enter(target);
        }
    }

    /** Open the menu beside its trigger, or at a point for a context menu, focusing an item. */
    open(focus: MenuFocus, point?: Point): void {
        // remember what to focus before opening and joining the parent's open submenus
        this.#focus = focus;
        this.#setPoint(point);
        this.#setOpen(true);
        if (this.parent !== null) {
            this.parent.#children.add(this);
        }
    }

    /** Close the menu and its submenus, returning the focus to its trigger once it hides when asked. */
    close(isReturningFocus: boolean): void {
        // close the submenus first
        this.closeSubmenus();

        // close the menu
        this.#isReturningFocus = isReturningFocus;
        this.#setOpen(false);
        if (this.parent !== null) {
            this.parent.#children.delete(this);
        }
    }

    /** Close the open submenus of this menu. */
    closeSubmenus(): void {
        for (const child of this.#children) {
            child.close(false);
        }
        this.#children.clear();
    }

    /** Follow the platform closing the menu, such as on a click outside. */
    follow(event: ToggleEvent): void {
        if (event.newState === "closed" && this.isOpen()) {
            this.close(false);
        }
    }

    /** Add an item to the menu until it unmounts, returning the attributes that take its element and follow its focus. */
    join(isDisabled: Accessor<boolean>): MenuItemJoin {
        // read the item's text for typeahead from its element
        const key = createUniqueId();
        let element: HTMLElement | undefined;
        this.list.add({
            key,
            text: () => element?.textContent?.trim() ?? "",
            isDisabled,
            element: () => element,
        });

        return {
            ref: (target) => {
                element = target;
            },
            onFocus: () => this.list.focus.focusIn(key),
        };
    }

    /** Read the top menu of a chain of submenus. */
    root(): MenuControl {
        return this.parent === null ? this : this.parent.root();
    }
}

/** The reference that takes a menu item's element, and the handler that follows its focus. */
export interface MenuItemJoin extends Record<"ref", (element: HTMLElement) => void> {
    /** Follow the focus onto the item. */
    readonly onFocus: () => void;
}

/** The properties of a top menu's root. */
export interface MenuRootProperties {
    /** Whether the menu is open, which makes the open state controlled. */
    readonly open?: boolean;
    /** Whether the menu starts open when its state is uncontrolled. */
    readonly defaultOpen?: boolean;
    /** Handle the menu opening or closing. */
    readonly onOpenChange?: (open: boolean) => void;
    /** The trigger and content. */
    readonly children?: JSX.Element;
}

/** The selected value of a radio group in a menu. */
export interface MenuRadioControl {
    /** The selected value. */
    readonly value: Accessor<string | undefined>;
    /** Select a value. */
    readonly select: (value: string) => void;
}

/** The properties of an element of a menu, the native element's attributes included. */
export type MenuElementProperties<Target extends HTMLElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> &
    ElementPartProperties;

/** The properties of a menu's content. */
export interface MenuContentProperties extends Omit<
    MenuElementProperties<HTMLDivElement>,
    "ref" | "onKeyDown" | "onToggle"
> {
    /** The side of the trigger it opens on, bottom by default. */
    readonly side?: Side;
    /** The edge of the trigger it lines up with, start by default. */
    readonly align?: Align;
    /** The gap from the trigger in pixels, 4 by default. */
    readonly sideOffset?: number;
}

/** The properties of a menu item. */
export interface MenuItemProperties extends Omit<
    MenuElementProperties<HTMLDivElement>,
    "onClick" | "onKeyDown" | "onPointerEnter" | "onFocus" | "ref"
> {
    /** Handle the item being chosen, which closes the menu unless the handler prevents the default. */
    readonly onSelect?: (event: Event) => void;
    /** Whether the item is unavailable. */
    readonly disabled?: boolean;
    /** Whether the item leaves room for an indicator at its start. */
    readonly inset?: boolean;
    /** The look of the item, destructive for an action that deletes. */
    readonly variant?: "default" | "destructive";
    /** Render another element with the item's attributes, such as a link, the menu's own element by default. */
    readonly render?: Render;
}

/** The properties of a menu item that checks and unchecks. */
export interface MenuCheckboxItemProperties extends MenuItemProperties {
    /** Whether the item is checked. */
    readonly checked: boolean;
    /** Handle the item being checked or unchecked. */
    readonly onCheckedChange?: (checked: boolean) => void;
}

/** The properties of a radio group of a menu. */
export interface MenuRadioGroupProperties extends MenuElementProperties<HTMLDivElement> {
    /** The selected value. */
    readonly value: string | undefined;
    /** Handle another value being selected. */
    readonly onValueChange?: (value: string) => void;
}

/** The properties of a radio item of a menu. */
export interface MenuRadioItemProperties extends MenuItemProperties {
    /** The value the item stands for. */
    readonly value: string;
}

/** Read the menu of the nearest menu, refusing elements outside one. */
export function useMenu(): MenuControl {
    const control = useContext(MenuContext);
    if (control === null) {
        throw new TypeError("menu elements need a menu around them");
    }

    return control;
}

/** Hold the open state of a top menu, or of a submenu of the nearest menu without one passed. */
export function Menu(properties: {
    readonly control?: MenuControl;
    readonly children?: JSX.Element;
}): JSX.Element {
    const control = properties.control ?? new MenuControl(useMenu());

    return <MenuContext value={control}>{properties.children}</MenuContext>;
}

/** Render the menu in the top layer, which arrow keys, Home, End and typed letters move through. */
export function MenuContent(properties: MenuContentProperties): JSX.Element {
    // read the menu, its placement and the text direction
    const control = useMenu();
    const locale = useLocale();
    const content = merge(DEFAULTS, properties);
    const rest = omit(content, "side", "align", "sideOffset", "xstyle", "style");
    createEffect(control.isOpen, (isOpen) => control.sync(isOpen));

    // place a context menu at its point
    const position = (): style.Styles => {
        const point = control.point();

        return point === undefined ? null : Position.at(point);
    };

    return (
        <TopLayer>
            <div
                id={control.id}
                popover="auto"
                role="menu"
                tabindex={-1}
                aria-labelledby={control.point() === undefined ? control.triggerId : undefined}
                data-slot="menu-content"
                data-side={content.side}
                data-align={content.align}
                {...rest}
                ref={(element) => control.setContent(element)}
                onToggle={(event) => control.follow(event)}
                onKeyDown={(event) => navigate(event, control, locale.direction)}
                onFocusOut={(event) => control.list.focus.focusOut(event)}
                {...style.attributes(
                    [
                        text.footnote,
                        styles.content,
                        Position.beside(content.side, content.align),
                        Position.offset(content.sideOffset),
                        position(),
                        content.xstyle,
                    ],
                    content.style,
                )}
            />
        </TopLayer>
    );
}

/** Render an item that runs its action and closes every menu when chosen with a click, Enter or Space. */
export function MenuItem(properties: MenuItemProperties): JSX.Element {
    // join the menu and report the item's state over its styles
    const control = useMenu();
    const rest = omit(
        properties,
        "onSelect",
        "disabled",
        "inset",
        "variant",
        "xstyle",
        "style",
        "render",
    );
    const item = control.join(() => properties.disabled === true);
    const part: PartAttributes = merge(
        {
            role: "menuitem" as const,
            tabindex: -1,
            ref: item.ref,
            onFocus: item.onFocus,
            "data-menu-item": "",
            "data-slot": "menu-item",
            get "data-variant"() {
                return properties.variant ?? "default";
            },
            get "aria-disabled"() {
                return properties.disabled === true ? "true" : undefined;
            },
            get "data-disabled"() {
                return properties.disabled === true ? "" : undefined;
            },
            onClick: (event: MouseEvent) => choose(event, control, properties),
            onKeyDown: activate,
            onPointerEnter: (event: PartEvent<PointerEvent>) => hover(event, control),
        },
        () =>
            style.attributes(
                [
                    styles.item,
                    properties.variant === "destructive" && styles.destructive,
                    properties.inset === true && styles.inset,
                    properties.disabled === true && styles.disabled,
                    properties.xstyle,
                ],
                properties.style,
            ),
    );

    return rendered(properties.render, part, rest, () => <div {...part} {...rest} />);
}

/** Render an item that checks and unchecks, reporting its state through `aria-checked`. */
export function MenuCheckboxItem(properties: MenuCheckboxItemProperties): JSX.Element {
    // join the menu, reporting the check over the item's styles
    const control = useMenu();
    const rest = omit(
        properties,
        "checked",
        "onCheckedChange",
        "onSelect",
        "disabled",
        "inset",
        "variant",
        "xstyle",
        "style",
        "children",
    );
    const item = control.join(() => properties.disabled === true);

    return (
        <div
            role="menuitemcheckbox"
            tabindex={-1}
            aria-checked={properties.checked ? "true" : "false"}
            aria-disabled={properties.disabled === true ? "true" : undefined}
            data-state={properties.checked ? "checked" : "unchecked"}
            data-disabled={properties.disabled === true ? "" : undefined}
            data-menu-item=""
            data-slot="menu-checkbox-item"
            {...rest}
            ref={item.ref}
            onFocus={item.onFocus}
            onClick={(event) => {
                // flip the check of an enabled item and run its action
                if (properties.disabled !== true) {
                    properties.onCheckedChange?.(!properties.checked);
                }
                choose(event, control, properties);
            }}
            onKeyDown={(event) => activate(event)}
            onPointerEnter={(event) => hover(event, control)}
            {...style.attributes(
                [
                    styles.item,
                    styles.inset,
                    properties.disabled === true && styles.disabled,
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            <span {...style.attrs(styles.indicator)}>
                <Show when={properties.checked}>
                    <Icon name="check" />
                </Show>
            </span>
            {properties.children}
        </div>
    );
}

/** Render a group of radio items that share one selected value. */
export function MenuRadioGroup(properties: MenuRadioGroupProperties): JSX.Element {
    const rest = omit(properties, "value", "onValueChange", "xstyle", "style");
    const radio: MenuRadioControl = {
        value: () => properties.value,
        select: (value) => properties.onValueChange?.(value),
    };

    return (
        <MenuRadioContext value={radio}>
            <div
                role="group"
                data-slot="menu-radio-group"
                {...rest}
                {...style.attributes([properties.xstyle], properties.style)}
            />
        </MenuRadioContext>
    );
}

/** Render an item of a radio group, checked when its value is the group's. */
export function MenuRadioItem(properties: MenuRadioItemProperties): JSX.Element {
    // join the menu and its radio group, refusing an item outside a group
    const control = useMenu();
    const radio = useContext(MenuRadioContext);
    if (radio === null) {
        throw new TypeError("a menu radio item needs a menu radio group around it");
    }
    const rest = omit(
        properties,
        "value",
        "onSelect",
        "disabled",
        "inset",
        "variant",
        "xstyle",
        "style",
        "children",
    );
    const isChecked = (): boolean => radio.value() === properties.value;
    const item = control.join(() => properties.disabled === true);

    return (
        <div
            role="menuitemradio"
            tabindex={-1}
            aria-checked={isChecked() ? "true" : "false"}
            aria-disabled={properties.disabled === true ? "true" : undefined}
            data-state={isChecked() ? "checked" : "unchecked"}
            data-disabled={properties.disabled === true ? "" : undefined}
            data-menu-item=""
            data-slot="menu-radio-item"
            {...rest}
            ref={item.ref}
            onFocus={item.onFocus}
            onClick={(event) => {
                // select the value of an enabled item and run its action
                if (properties.disabled !== true) {
                    radio.select(properties.value);
                }
                choose(event, control, properties);
            }}
            onKeyDown={(event) => activate(event)}
            onPointerEnter={(event) => hover(event, control)}
            {...style.attributes(
                [
                    styles.item,
                    styles.inset,
                    properties.disabled === true && styles.disabled,
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            <span {...style.attrs(styles.indicator)}>
                <Show when={isChecked()}>
                    <Icon name="circle" weight="fill" size="0.5em" />
                </Show>
            </span>
            {properties.children}
        </div>
    );
}

/** Render a label over a group of items. */
export function MenuLabel(
    properties: MenuElementProperties<HTMLDivElement> & { readonly inset?: boolean },
): JSX.Element {
    const rest = omit(properties, "inset", "xstyle", "style");

    return (
        <div
            data-slot="menu-label"
            {...rest}
            {...style.attributes(
                [styles.label, properties.inset === true && styles.inset, properties.xstyle],
                properties.style,
            )}
        />
    );
}

/** Render a group of related items. */
export function MenuGroup(properties: MenuElementProperties<HTMLDivElement>): JSX.Element {
    return renderPart("div", "menu-group", properties, null, { role: "group" });
}

/** Render a line between groups of items. */
export function MenuSeparator(properties: MenuElementProperties<HTMLDivElement>): JSX.Element {
    return renderPart("div", "menu-separator", properties, styles.separator, { role: "separator" });
}

/** Render the keyboard shortcut of an item at its end. */
export function MenuShortcut(properties: MenuElementProperties<HTMLSpanElement>): JSX.Element {
    return renderPart("span", "menu-shortcut", properties, [text.caption, styles.shortcut]);
}

/** Render the item that opens a submenu on hover, Enter, Space or the arrow key toward it. */
export function MenuSubTrigger(
    properties: Omit<MenuItemProperties, "onSelect" | "variant">,
): JSX.Element {
    // read the submenu and the direction its arrow key points in
    const control = useMenu();
    const locale = useLocale();
    const rest = omit(properties, "disabled", "inset", "xstyle", "style", "children");
    const isEnabled = (): boolean => properties.disabled !== true;
    const parent = control.parent;
    if (parent === null) {
        throw new TypeError("a menu sub trigger needs a submenu around it");
    }
    const item = parent.join(() => !isEnabled());
    const opens = (key: string): boolean =>
        key === "Enter" ||
        key === " " ||
        key === (locale.direction === "rtl" ? "ArrowLeft" : "ArrowRight");

    return (
        <div
            id={control.triggerId}
            role="menuitem"
            tabindex={-1}
            aria-haspopup="menu"
            aria-expanded={control.isOpen() ? "true" : "false"}
            aria-controls={control.id}
            aria-disabled={properties.disabled === true ? "true" : undefined}
            data-menu-item=""
            data-slot="menu-sub-trigger"
            {...rest}
            ref={[item.ref, (element: HTMLElement) => control.setTrigger(element)]}
            onFocus={item.onFocus}
            onClick={() => isEnabled() && control.open("first")}
            onKeyDown={(event) => {
                // open the submenu of an enabled trigger and focus its first item
                if (isEnabled() && opens(event.key)) {
                    event.preventDefault();
                    event.stopPropagation();
                    control.open("first");
                }
            }}
            onPointerEnter={(event) => {
                // focus an enabled trigger and open its submenu in place of any other
                if (isEnabled()) {
                    event.currentTarget.focus();
                    parent.closeSubmenus();
                    control.open("none");
                }
            }}
            {...style.attributes(
                [
                    styles.item,
                    properties.inset === true && styles.inset,
                    properties.disabled === true && styles.disabled,
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            {properties.children}
            <span {...style.attrs(styles.chevron)}>
                <Icon icon={locale.direction === "rtl" ? caretLeft : caretRight} />
            </span>
        </div>
    );
}

/** Move the focus within a menu, close it, or hand a horizontal arrow key to its menubar. */
function navigate(
    event: KeyboardEvent & { readonly currentTarget: HTMLDivElement },
    control: MenuControl,
    direction: Direction,
): void {
    // leave keys from a submenu to the submenu
    if (
        event.target instanceof Element &&
        event.target.closest("[role=menu]") !== event.currentTarget
    ) {
        return;
    }

    // close on Escape, on the arrow key back out of a submenu, and on Tab
    const back = direction === "rtl" ? "ArrowRight" : "ArrowLeft";
    if (event.key === "Escape" || (event.key === back && control.parent !== null)) {
        event.preventDefault();
        event.stopPropagation();
        control.close(true);
    } else if (event.key === "Tab") {
        control.root().close(false);
    }
    // hand left and right in a top menu to its menubar
    else if ((event.key === "ArrowLeft" || event.key === "ArrowRight") && control.parent === null) {
        control.onCross?.(event, direction);
    }
    // move with the arrow keys, Home, End and typed letters
    else {
        control.list.focus.move(event, direction);
    }
}

/** Focus a hovered item and close the submenus of its menu. */
function hover(event: PartEvent<PointerEvent>, control: MenuControl): void {
    event.currentTarget.focus();
    control.closeSubmenus();
}

/** Click an item on Enter and Space, as buttons do. */
function activate(event: PartEvent<KeyboardEvent>): void {
    if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        event.currentTarget.click();
    }
}

/** Run an item's action and close every menu unless the action prevents it. */
function choose(
    event: MouseEvent,
    control: MenuControl,
    properties: Pick<MenuItemProperties, "onSelect" | "disabled">,
): void {
    // ignore a disabled item
    if (properties.disabled === true) {
        return;
    }

    // run the action before closing the chain of menus and returning the focus to its trigger
    properties.onSelect?.(event);
    if (!event.defaultPrevented) {
        control.root().close(true);
    }
}
