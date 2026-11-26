import { state } from "@destack/theme/state";
import { Icon } from "@destack/ui/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import {
    color,
    radius,
    shadow,
    space,
    stroke,
    weight,
    size,
    swatch,
} from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    flush,
    useActions,
    useAction,
    type Accessor,
    type ActionState,
    createContext,
    createEffect,
    createSignal,
    createUniqueId,
    For,
    onCleanup,
    type JSX,
    merge,
    omit,
    type Setter,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { present } from "@destack/schema";
import { Timeout } from "@destack/view/primitives/timer";
import { Hover, type HoverEvent, SafePolygon } from "../interaction/index.ts";
import {
    contentOr,
    type ElementPartProperties,
    type PartEvent,
    type Render,
    renderPart,
    mergeProperties,
} from "../part/index.ts";
import { type Direction, Shortcut, t } from "@destack/locale";
import { type Action, type ActionInputs, field, type ValuesOf } from "@destack/object";
import { ChoiceIcon } from "../action/choice-icon.tsx";
import type { IconBodies } from "@destack/icon";
import { useCommandPlatform } from "../kbd/index.ts";
import { ListState, tabbablesIn } from "../focus/index.ts";
import {
    type Anchor,
    type Point,
    Position,
    PositionAnchor,
    ANCHOR_PROPERTIES,
} from "../position/index.ts";
import {
    Dismissal,
    OpenChange,
    type OpenChildren,
    type OpenProperties,
    OpenState,
    type OpenTrigger,
    TopLayer,
} from "../layer/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The wait before a hovered trigger opens its submenu, in milliseconds. */
const SUBMENU_DELAY = 100;

/** The distance a held pointer moves before letting it go chooses an item, in pixels. */
const DRAG_DISTANCE = 4;

/** The time a pointer is held before letting it go chooses an item, in milliseconds. */
const DRAG_TIME = 200;

/** The time after a release in which the platform may dismiss the menu, in milliseconds. */
const KEEP_TIME = 100;

/** The properties people change in a menu's controls and inspector. */
const MENU_CONTENT_PROPERTIES = {
    side: ANCHOR_PROPERTIES.side.default("bottom"),
    align: ANCHOR_PROPERTIES.align.default("start"),
};

/** The properties people change in a menu item's controls and inspector. */
const MENU_ITEM_PROPERTIES = {
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

/** The properties people change in a checking menu item's controls and inspector. */
const MENU_CHECK_ITEM_PROPERTIES = {
    disabled: MENU_ITEM_PROPERTIES.disabled,
};

/** The menu of the nearest menu, null outside one. */
const MenuContext = createContext<MenuControl | null>(null);

/** The selected value of the nearest radio group of a menu, null outside one. */
const MenuRadioContext = createContext<MenuRadioControl | null>(null);

/** The item around a shortcut, which takes the shortcut's keys. */
const MenuItemContext = createContext<MenuItemJoin | null>(null);

/** A shortcut whose item has a check, which pushes both to the end. */
const BEFORE_CHECK = ":is(:has(~ [data-slot=menu-indicator]))";

/** A mark after an item's shortcut, which the shortcut pushes to the end. */
const AFTER_SHORTCUT = ":is([data-slot=menu-shortcut] ~ *)";

/** The styles of a menu and its elements. */
const styles = style.create({
    content: {
        minWidth: `calc(10 * ${space[4]})`,
        maxWidth: `calc(100vw - 2 * ${space[1]})`,
        maxHeight: `calc(100dvh - 2 * ${space[1]})`,
        overflowY: "auto",
        inset: "auto",
        padding: space[1],
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.popover,
        color: color.foreground,
        boxShadow: shadow.overlay,
    },
    submenu: {
        marginBlockStart: `calc(-1 * (${space[1]} + ${stroke.border}))`,
    },
    item: {
        position: "relative",
        display: "flex",
        alignItems: "center",
        minHeight: size[2],
        gap: space[2],
        paddingBlock: space[1],
        paddingInline: space[2],
        borderRadius: radius[2],
        outlineStyle: "none",
        backgroundColor: {
            default: "transparent",
            ":focus": { default: color.selected, [media.forcedColors]: "Highlight" },
        },
        color: {
            default: "inherit",
            ":focus": { default: color.foreground, [media.forcedColors]: "HighlightText" },
        },
        forcedColorAdjust: {
            default: null,
            ":focus": { default: null, [media.forcedColors]: "none" },
        },
        [color.mutedForeground]: {
            default: null,
            ":focus": { default: null, [media.forcedColors]: "HighlightText" },
        },
        cursor: "default",
        userSelect: "none",
    },
    open: {
        backgroundColor: { default: color.selected, [media.forcedColors]: "Highlight" },
        color: { default: color.foreground, [media.forcedColors]: "HighlightText" },
        forcedColorAdjust: { default: null, [media.forcedColors]: "none" },
        [color.mutedForeground]: { default: null, [media.forcedColors]: "HighlightText" },
    },
    destructive: {
        color: { default: swatch.destructiveText, ":focus": swatch.destructiveText },
        backgroundColor: {
            default: "transparent",
            ":focus": swatch.destructiveTint,
        },
    },
    indicator: {
        display: "inline-flex",
        marginInlineStart: "auto",
        color: color.mutedForeground,
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
        // show after a check, so that the keys of every item line up
        order: 1,
        marginInlineStart: { default: "auto", [BEFORE_CHECK]: 0 },
        color: color.mutedForeground,
        letterSpacing: "0.1em",
    },
    chevron: {
        // show after the keys
        order: 2,
        marginInlineStart: {
            default: "auto",
            [AFTER_SHORTCUT]: 0,
        },
        display: "inline-flex",
    },
});

/** The item a newly opened menu focuses. */
export type MenuFocus = "first" | "last" | "none";

/** A pointer held down since it opened a menu, which chooses the item it is let go over. */
export class MenuRelease {
    /** The menu the pointer opened. */
    readonly #control: MenuControl;
    /** Stop waiting for the pointer to be let go. */
    #stop: () => void;
    /** Whether the next click is the one the press that opened the menu leaves behind. */
    #isClickSwallowed: boolean;
    /** Whether the menu outlasts the platform's dismissal. */
    #isKeeping: boolean;
    /** The time the menu outlasts it. */
    readonly #timer: Timeout;

    /** Create a release of a menu, waiting for no pointer. */
    constructor(control: MenuControl) {
        // start with the pointer up
        this.#control = control;
        this.#stop = () => undefined;
        this.#isClickSwallowed = false;
        this.#isKeeping = false;
        this.#timer = new Timeout();
        onCleanup(() => this.#stop());
    }

    /** Open the menu as a mouse presses its trigger, leaving a finger and other buttons alone. */
    press(event: PointerEvent, trigger?: OpenTrigger): void {
        // leave a press that closes the menu to its click
        this.#isClickSwallowed = false;
        const isPlain = event.button === 0 && !event.ctrlKey;
        const isOpen = untrack(() => this.#control.state.isFrom(trigger?.element));
        if (event.pointerType !== "mouse" || !isPlain || isOpen) {
            return;
        }

        // open it and wait for the pointer
        this.#isClickSwallowed = true;
        this.#control.open("first", new OpenChange("trigger-press", event, trigger));
        this.await(event);
    }

    /** Open or close the menu on a click, unless its press opened it. */
    click(event: MouseEvent, trigger?: OpenTrigger): void {
        // leave the click of the press that opened the menu alone, never a key's
        const isSwallowed = this.#isClickSwallowed && event.detail !== 0;
        this.#isClickSwallowed = false;
        if (isSwallowed) {
            this.#control.keep();

            return;
        }

        // open or close the menu
        const change = new OpenChange("trigger-press", event, trigger);
        if (untrack(() => this.#control.state.isFrom(trigger?.element))) {
            this.#control.close(true, change);
        } else {
            this.#control.open("first", change);
        }
    }

    /** Wait for a held pointer to be let go: over an item it chooses it, elsewhere it closes. */
    await(event: Pick<PointerEvent, "clientX" | "clientY" | "timeStamp" | "view">): void {
        // follow one pointer at a time
        this.#stop();
        const document = event.view?.document;
        const up = (released: PointerEvent) => {
            // leave a click in place, which moves and takes little, alone
            this.#stop();
            const distance = Math.hypot(
                released.clientX - event.clientX,
                released.clientY - event.clientY,
            );
            const isClick =
                distance < DRAG_DISTANCE || released.timeStamp - event.timeStamp < DRAG_TIME;
            if (isClick || !(released.target instanceof Element)) {
                this.#keep();

                return;
            }

            // choose the item under it, else close the menu unless a trigger or menu is under it
            const item = released.target.closest<HTMLElement>("[data-menu-item]");
            const isWithin = released.target.closest("[role=menu], [aria-haspopup=menu]") !== null;
            if (item !== null) {
                item.click();
            } else if (isWithin) {
                this.#keep();
            } else {
                this.#control.close(true, new OpenChange("outside-press", released));
            }
        };
        document?.addEventListener("pointerup", up, true);
        this.#stop = () => document?.removeEventListener("pointerup", up, true);
    }

    /** Report whether the menu outlasts the platform's dismissal. */
    isKeeping(): boolean {
        return this.#isKeeping;
    }

    /** Let the menu outlast the platform's dismissal for a moment. */
    #keep(): void {
        this.#isKeeping = true;
        this.#timer.set(() => (this.#isKeeping = false), KEEP_TIME);
    }
}

/** The open state, elements and focus of a menu or submenu. */
export class MenuControl<Payload = unknown> {
    /** The id of the menu element. */
    readonly id: string;
    /** The open state, and the trigger it opened from. */
    readonly state: OpenState<Payload>;
    /** The menu this one is a submenu of, null for a top menu. */
    readonly parent: MenuControl | null;
    /** Whether the menu is open. */
    readonly isOpen: Accessor<boolean>;
    /** The point a context menu opens at, undefined for a menu that opens beside its trigger. */
    readonly point: Accessor<Point | undefined>;
    /** The menu's own items in document order. */
    readonly list: ListState;
    /** Choose the item a pointer that opened the menu is let go over, after a drag to it. */
    readonly release: MenuRelease;
    /** Handle a horizontal arrow key a top menu leaves unused. */
    onCross: ((event: KeyboardEvent, direction: Direction) => void) | undefined;
    /** The element the menu is placed beside in place of its trigger. */
    #anchor: HTMLElement | undefined;
    /** The menu element. */
    #content: HTMLElement | undefined;
    /** The item to focus once the menu shows. */
    #focus: MenuFocus;
    /** Whether the trigger takes the focus back once the menu hides. */
    #isReturningFocus: boolean;
    /** Stop placing the shown menu beside its trigger. */
    #unplace: () => void;
    /** The way of a pointer from a submenu's trigger to its submenu. */
    readonly polygon: SafePolygon;
    /** The pending open of a submenu whose trigger is hovered. */
    readonly #timer: Timeout;
    /** The open submenus. */
    readonly #children: Set<MenuControl>;
    /** Why the platform closes the menu. */
    readonly #dismissal: Dismissal;
    /** Replace the point a context menu opens at. */
    readonly #setPoint: Setter<Point | undefined>;

    /** Create a closed menu, a submenu when it has a parent. */
    constructor(parent: MenuControl | null, options: OpenProperties = {}) {
        // follow the controlled state, else the menu's own, without elements yet
        this.id = createUniqueId();
        this.state = new OpenState(options, this.id);
        this.#dismissal = new Dismissal();
        const [point, setPoint] = createSignal<Point | undefined>(undefined);
        this.parent = parent;
        this.isOpen = this.state.isOpen;
        this.point = point;
        this.list = new ListState({ orientation: "vertical", isLooping: true, isTypeahead: true });
        this.onCross = undefined;
        this.#anchor = undefined;
        this.#content = undefined;
        this.#focus = "first";
        this.#isReturningFocus = false;
        this.#unplace = () => undefined;
        this.#children = new Set();
        this.polygon = new SafePolygon();
        this.release = new MenuRelease(this);
        this.#timer = new Timeout();
        this.#setPoint = setPoint;

        // stop placing the overlay and drop its anchor once its owner goes away
        onCleanup(() => {
            this.#dismissal.stop();
            this.#unplace();
        });
    }

    /** Set the element that opens the menu and anchors it. */
    setTrigger(element: HTMLElement): void {
        this.state.setTrigger(element);
    }

    /** Set the element the menu is placed beside in place of its trigger. */
    setAnchor(element: HTMLElement | undefined): void {
        this.#anchor = element;
    }

    /** Set the menu element and show or hide it as the menu opens and closes. */
    setContent(element: HTMLElement): void {
        this.#content = element;
    }

    /** Show or hide the menu element for an open state. */
    sync(isOpen: boolean): void {
        // wait for the menu element to mount
        const content = this.#content;
        if (content === undefined) {
            return;
        }

        // show it beside its trigger, or at its point, and focus the item asked for
        this.state.mark(content);
        if (isOpen) {
            const trigger = untrack(this.state.trigger)?.element;
            const point = untrack(this.point);
            const anchor = this.#anchor ?? trigger;
            const isBeside = anchor !== undefined && point === undefined;
            const detach = isBeside ? Position.attach(content, anchor, this.id) : () => undefined;
            content.showPopover(
                isBeside && trigger !== undefined ? { source: trigger } : undefined,
            );
            const unplace = isBeside ? Position.place(content, anchor) : () => undefined;
            const unfit = point === undefined ? () => undefined : Position.fit(content, point);
            this.#unplace = () => {
                detach();
                unplace();
                unfit();
            };
            this.#dismissal.start(content);
            untrack(() => this.#focusItem(content));
        } else {
            this.#dismissal.stop();
            this.#unplace();
            content.hidePopover();
            if (this.#isReturningFocus) {
                focusIn(untrack(this.state.trigger)?.element);
            }
        }
        this.#isReturningFocus = false;
    }

    /** Close the menu as the trigger it is open from goes away. */
    drop(element: HTMLElement | undefined): void {
        if (element !== undefined && untrack(() => this.isOpen() && this.state.isFrom(element))) {
            this.close(false, new OpenChange("none", undefined, undefined, false));
        }
    }

    /** Read where the top menu opened: its trigger, null without one. */
    origin(): Element | null {
        return this.root().state.trigger()?.element ?? null;
    }

    /** Show an open menu again, as the platform dismisses one that opened during a press. */
    keep(): void {
        if (untrack(this.isOpen) && this.#content?.matches(":popover-open") === false) {
            this.#unplace();
            this.sync(true);
        }
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
    open(focus: MenuFocus, change: OpenChange<Payload> = new OpenChange(), point?: Point): void {
        // remember what to focus before opening and joining the parent's open submenus
        this.#focus = focus;
        this.#setPoint(point);

        // render the items for where it opens, so it measures and focuses those, then show it
        this.state.show(change);
        if (this.parent !== null) {
            this.parent.#children.add(this);
        }
    }

    /** Close the menu and its submenus. */
    close(isReturningFocus: boolean, change: OpenChange<Payload> = new OpenChange()): void {
        // close the submenus first
        this.closeSubmenus();

        // close the menu and forget where the pointer headed
        this.#timer.clear();
        this.polygon.stop();
        this.#isReturningFocus = isReturningFocus;
        this.state.set(false, change);
        if (this.parent !== null) {
            this.parent.#children.delete(this);
        }
    }

    /** Focus a hovered item in place of an open submenu, unless the pointer heads for that one. */
    hover(element: HTMLElement, applies: () => boolean): void {
        this.polygon.defer(applies, () => {
            element.focus();
            this.closeSubmenus();
        });
    }

    /** Open the submenu of a hovered trigger a moment later, in place of any other. */
    hoverTrigger(element: HTMLElement, applies: () => boolean): void {
        // return to the trigger of an open submenu
        const parent = present(this.parent, "the submenu's parent menu");
        if (untrack(this.isOpen)) {
            parent.polygon.stop();
            element.focus();

            return;
        }

        // close the other submenus once the pointer stops heading for one
        parent.polygon.defer(applies, () => {
            element.focus();
            parent.closeSubmenus();
            this.#timer.set(() => this.open("none"), SUBMENU_DELAY);
        });
    }

    /** Let the pointer leave the trigger of a submenu on its way to it. */
    leaveTrigger(event: HoverEvent): void {
        this.#timer.clear();
        const parent = present(this.parent, "the submenu's parent menu");
        parent.polygon.enter(event, untrack(this.isOpen) ? this.#content : undefined);
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
        // leave a menu its state closed, and keep one that opened during a press
        const content = this.#content;
        if (event.newState !== "closed" || content === undefined || !untrack(this.isOpen)) {
            return;
        }
        const trigger = untrack(this.state.trigger)?.element;
        const change = this.#dismissal.change(content, trigger);
        const isKeeping = this.release.isKeeping();
        if (!isKeeping) {
            this.close(false, change);
            flush();
        }

        // show it again where it stays open
        if (isKeeping || change.isCancelable) {
            this.keep();
        }
    }

    /** Add an item to the menu until it unmounts. */
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

        // read the keys of the shortcut the item shows
        const [keys, showKeys] = createSignal<string | undefined>(undefined, { ownedWrite: true });

        return {
            ref: (target) => {
                element = target;
            },
            onFocus: () => this.list.focus.focusIn(key),
            keys,
            showKeys,
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
    /** The keys of the shortcut the item shows, as `aria-keyshortcuts` writes them. */
    readonly keys: Accessor<string | undefined>;
    /** Show the keys of a shortcut in the item, none when absent. */
    readonly showKeys: (keys: string | undefined) => void;
}

/** The properties of a top menu's root. */
export interface MenuRootProperties<Payload = unknown> extends OpenProperties {
    /** The menu that triggers elsewhere open, which holds the open state in the root's place. */
    readonly control?: MenuControl<Payload>;
    /** The trigger and content, or what it shows for the trigger it opened from. */
    readonly children?: OpenChildren<Payload>;
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

/** The properties of an item's shortcut, the native element's attributes included. */
export interface MenuShortcutProperties extends Omit<
    MenuElementProperties<HTMLSpanElement>,
    "children"
> {
    /** The keys, as a keybinding writes them, such as `mod+r`. */
    readonly keys: string;
}

/** The properties of a menu's content besides its declared ones: its anchor and gap. */
export interface MenuContentAttributes extends MenuElementProperties<HTMLDivElement> {
    /** What it is placed beside in place of its trigger: an element, or a box in the viewport. */
    readonly anchor?: Anchor | undefined;
    /** The gap from the trigger in pixels, 4 by default. */
    readonly sideOffset?: number;
}

/** The properties of a menu item besides its declared ones: its choice and element. */
export interface MenuItemAttributes extends MenuElementProperties<HTMLDivElement> {
    /** Handle the item being chosen. */
    readonly onSelect?: (event: Event) => void;
    /** Render another element with the item's attributes, such as a link. */
    readonly render?: Render;
}

/** The properties of a menu item that checks besides its declared ones: its check. */
export interface MenuCheckboxItemAttributes extends MenuItemAttributes {
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

/** The properties of a menu's radio item besides its declared ones: its value. */
export interface MenuRadioItemAttributes extends MenuItemAttributes {
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
export function Menu<Payload = unknown>(properties: {
    readonly control?: MenuControl<Payload>;
    readonly children?: OpenChildren<Payload>;
}): JSX.Element {
    const control = untrack(() => properties.control) ?? new MenuControl<Payload>(useMenu());

    return <MenuContext value={control}>{control.state.render(properties.children)}</MenuContext>;
}

/** Render the menu in the top layer, which arrow keys, Home, End and typed letters move through. */
export const MenuContent = defineComponent({
    name: "menuContent",
    properties: MENU_CONTENT_PROPERTIES,
    render(
        properties: RenderProperties<MenuContentAttributes, typeof MENU_CONTENT_PROPERTIES>,
    ): JSX.Element {
        // read the menu, its placement and the text direction
        const control = useMenu();
        const locale = useLocale();
        const rest = omit(properties, "anchor", "side", "align", "sideOffset");

        // show and hide the menu as it opens and closes
        createEffect(control.isOpen, (isOpen) => untrack(() => control.sync(isOpen)));

        // place a context menu at its point
        const position = (): style.Styles => {
            const point = control.point();

            return point === undefined ? null : Position.at(point);
        };

        return (
            <TopLayer>
                <PositionAnchor
                    anchor={properties.anchor}
                    onAnchor={(element) => control.setAnchor(element)}
                />
                {renderPart(
                    "div",
                    "menu-content",
                    rest,
                    () => [
                        text.sm,
                        styles.content,
                        Position.beside(properties.side, properties.align),
                        Position.offset(properties.side, properties.sideOffset),
                        control.parent !== null && styles.submenu,
                        position(),
                    ],
                    {
                        id: control.id,
                        popover: "auto" as const,
                        role: "menu" as const,
                        tabindex: -1,
                        get "aria-labelledby"() {
                            return control.point() === undefined
                                ? control.state.labelId()
                                : undefined;
                        },
                        get "data-side"() {
                            return properties.side;
                        },
                        get "data-align"() {
                            return properties.align;
                        },
                        ref: (element) => control.setContent(element),
                        onToggle: (event) => control.follow(event),
                        onKeyDown: (event) => navigate(event, control, locale.direction),
                        onFocusOut: (event) => control.list.focus.focusOut(event),
                    },
                )}
            </TopLayer>
        );
    },
});

/** The properties of a menu's content, its anchor and placement included. */
export type MenuContentProperties = ComponentProperties<typeof MenuContent>;

/** Render an item that runs its action and closes every menu. */
export const MenuItem = defineComponent({
    name: "menuItem",
    properties: MENU_ITEM_PROPERTIES,
    render(
        properties: RenderProperties<MenuItemAttributes, typeof MENU_ITEM_PROPERTIES>,
    ): JSX.Element {
        // join the menu and report the item's state over its styles
        const control = useMenu();
        const rest = omit(properties, "onSelect", "disabled", "variant");
        const item = control.join(() => properties.disabled);
        const hover: Hover = new Hover({
            onHoverStart: (event) => control.hover(event.target, () => hover.isHovered()),
        });

        return (
            <MenuItemContext value={item}>
                {renderPart(
                    "div",
                    "menu-item",
                    rest,
                    () => [
                        styles.item,
                        properties.variant === "destructive" && styles.destructive,
                        properties.disabled && state.inert,
                    ],
                    merge(hover.attributes(), {
                        role: "menuitem" as const,
                        tabindex: -1,
                        ref: item.ref,
                        onFocus: item.onFocus,
                        get "aria-keyshortcuts"() {
                            return item.keys();
                        },
                        "data-menu-item": "",
                        get "data-variant"() {
                            return properties.variant;
                        },
                        get "aria-disabled"() {
                            return properties.disabled ? "true" : undefined;
                        },
                        get "data-disabled"() {
                            return properties.disabled ? "" : undefined;
                        },
                        onClick: (event: MouseEvent) => choose(event, control, properties),
                        onKeyDown: activate,
                    }),
                )}
            </MenuItemContext>
        );
    },
});

/** The properties of a menu item, its look included. */
export type MenuItemProperties = ComponentProperties<typeof MenuItem>;

/** Render an item that checks and unchecks, reporting its state through `aria-checked`. */
export const MenuCheckboxItem = defineComponent({
    name: "menuCheckboxItem",
    properties: MENU_CHECK_ITEM_PROPERTIES,
    render(
        properties: RenderProperties<MenuCheckboxItemAttributes, typeof MENU_CHECK_ITEM_PROPERTIES>,
    ): JSX.Element {
        // join the menu, reporting the check over the item's styles
        const control = useMenu();
        const rest = omit(
            properties,
            "checked",
            "onCheckedChange",
            "onSelect",
            "disabled",
            "children",
        );
        const item = control.join(() => properties.disabled);
        const hover: Hover = new Hover({
            onHoverStart: (event) => control.hover(event.target, () => hover.isHovered()),
        });

        return (
            <MenuItemContext value={item}>
                {renderPart(
                    "div",
                    "menu-checkbox-item",
                    rest,
                    () => [styles.item, properties.disabled && state.inert],
                    mergeProperties(hover.attributes(), {
                        role: "menuitemcheckbox" as const,
                        tabindex: -1,
                        get "aria-keyshortcuts"() {
                            return item.keys();
                        },
                        get "aria-checked"() {
                            return properties.checked ? "true" : "false";
                        },
                        get "aria-disabled"() {
                            return properties.disabled ? "true" : undefined;
                        },
                        get "data-checked"() {
                            return properties.checked ? "" : undefined;
                        },
                        get "data-unchecked"() {
                            return properties.checked ? undefined : "";
                        },
                        get "data-disabled"() {
                            return properties.disabled ? "" : undefined;
                        },
                        "data-menu-item": "" as const,
                        ref: item.ref,
                        onFocus: item.onFocus,
                        onClick: (event: MouseEvent) => {
                            // flip the check of an enabled item and run its action
                            if (!properties.disabled) {
                                properties.onCheckedChange?.(!properties.checked);
                            }
                            choose(event, control, properties);
                        },
                        onKeyDown: (event: PartEvent<KeyboardEvent>) => activate(event),
                        get children() {
                            return (
                                <>
                                    {properties.children}
                                    <span
                                        data-slot="menu-indicator"
                                        {...style.attrs(styles.indicator)}
                                    >
                                        <Show when={properties.checked}>
                                            <Icon name="check" />
                                        </Show>
                                    </span>
                                </>
                            );
                        },
                    }),
                )}
            </MenuItemContext>
        );
    },
});

/** The properties of a menu item that checks and unchecks. */
export type MenuCheckboxItemProperties = ComponentProperties<typeof MenuCheckboxItem>;

/** Render a group of radio items that share one selected value. */
export function MenuRadioGroup(properties: MenuRadioGroupProperties): JSX.Element {
    const rest = omit(properties, "value", "onValueChange");
    const radio: MenuRadioControl = {
        value: () => properties.value,
        select: (value) => properties.onValueChange?.(value),
    };

    return (
        <MenuRadioContext value={radio}>
            {renderPart("div", "menu-radio-group", rest, null, { role: "group" as const })}
        </MenuRadioContext>
    );
}

/** Render an item of a radio group, checked when its value is the group's. */
export const MenuRadioItem = defineComponent({
    name: "menuRadioItem",
    properties: MENU_CHECK_ITEM_PROPERTIES,
    render(
        properties: RenderProperties<MenuRadioItemAttributes, typeof MENU_CHECK_ITEM_PROPERTIES>,
    ): JSX.Element {
        // join the menu and its radio group, refusing an item outside a group
        const control = useMenu();
        const radio = useContext(MenuRadioContext);
        if (radio === null) {
            throw new TypeError("a menu radio item needs a menu radio group around it");
        }
        const rest = omit(properties, "value", "onSelect", "disabled", "children");
        const isChecked = (): boolean => radio.value() === properties.value;
        const item = control.join(() => properties.disabled);
        const hover: Hover = new Hover({
            onHoverStart: (event) => control.hover(event.target, () => hover.isHovered()),
        });

        return (
            <MenuItemContext value={item}>
                {renderPart(
                    "div",
                    "menu-radio-item",
                    rest,
                    () => [styles.item, properties.disabled && state.inert],
                    mergeProperties(hover.attributes(), {
                        role: "menuitemradio" as const,
                        tabindex: -1,
                        get "aria-keyshortcuts"() {
                            return item.keys();
                        },
                        get "aria-checked"() {
                            return isChecked() ? "true" : "false";
                        },
                        get "aria-disabled"() {
                            return properties.disabled ? "true" : undefined;
                        },
                        get "data-checked"() {
                            return isChecked() ? "" : undefined;
                        },
                        get "data-unchecked"() {
                            return isChecked() ? undefined : "";
                        },
                        get "data-disabled"() {
                            return properties.disabled ? "" : undefined;
                        },
                        "data-menu-item": "" as const,
                        ref: item.ref,
                        onFocus: item.onFocus,
                        onClick: (event: MouseEvent) => {
                            // select the value of an enabled item and run its action
                            if (!properties.disabled) {
                                radio.select(properties.value);
                            }
                            choose(event, control, properties);
                        },
                        onKeyDown: (event: PartEvent<KeyboardEvent>) => activate(event),
                        get children() {
                            return (
                                <>
                                    {properties.children}
                                    <span
                                        data-slot="menu-indicator"
                                        {...style.attrs(styles.indicator)}
                                    >
                                        <Show when={isChecked()}>
                                            <Icon name="circle" weight="fill" size="0.5em" />
                                        </Show>
                                    </span>
                                </>
                            );
                        },
                    }),
                )}
            </MenuItemContext>
        );
    },
});

/** The properties of a radio item of a menu. */
export type MenuRadioItemProperties = ComponentProperties<typeof MenuRadioItem>;

/** Render a label over a group of items. */
export function MenuLabel(properties: MenuElementProperties<HTMLDivElement>): JSX.Element {
    const rest = properties;

    return renderPart("div", "menu-label", rest, styles.label);
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
export function MenuShortcut(properties: MenuShortcutProperties): JSX.Element {
    // name the keys on the item around it, and show them for sight alone
    const item = useContext(MenuItemContext);
    const isCommand = useCommandPlatform();
    createEffect(
        () => Shortcut.aria(properties.keys, isCommand()),
        (keys) => {
            item?.showKeys(keys);
        },
    );

    return renderPart(
        "span",
        "menu-shortcut",
        omit(properties, "keys"),
        [text.xs, styles.shortcut],
        {
            "aria-hidden": "true",
            get children() {
                return Shortcut.format(properties.keys, isCommand());
            },
        },
    );
}

/** Render the item that opens a submenu on hover, Enter, Space or the arrow key toward it. */
export function MenuSubTrigger(
    properties: Omit<MenuItemProperties, "onSelect" | "variant">,
): JSX.Element {
    // read the submenu and the direction its arrow key points in
    const control = useMenu();
    const locale = useLocale();
    const rest = omit(properties, "disabled", "children");
    const isEnabled = (): boolean => properties.disabled !== true;
    const parent = control.parent;
    if (parent === null) {
        throw new TypeError("a menu sub trigger needs a submenu around it");
    }
    const id = control.state.triggerId();
    const item = parent.join(() => !isEnabled());
    const opens = (key: string): boolean =>
        key === "Enter" ||
        key === " " ||
        key === (locale.direction === "rtl" ? "ArrowLeft" : "ArrowRight");

    // focus an enabled trigger under the pointer and open its submenu in place of any other
    const hover: Hover = new Hover({
        onHoverStart: (event) => {
            if (isEnabled()) {
                control.hoverTrigger(event.target, () => hover.isHovered());
            }
        },
        onHoverEnd: (event) => control.leaveTrigger(event),
    });

    return renderPart(
        "div",
        "menu-sub-trigger",
        rest,
        () => [
            styles.item,
            control.isOpen() && styles.open,
            properties.disabled === true && state.inert,
        ],
        mergeProperties(hover.attributes(), {
            id: id,
            role: "menuitem" as const,
            tabindex: -1,
            "aria-haspopup": "menu" as const,
            get "aria-expanded"() {
                return control.isOpen() ? "true" : "false";
            },
            "aria-controls": control.id,
            get "aria-disabled"() {
                return properties.disabled === true ? "true" : undefined;
            },
            "data-menu-item": "" as const,
            ref: [item.ref, (element: HTMLElement) => control.setTrigger(element)],
            onFocus: item.onFocus,
            onClick: () => isEnabled() && control.open("first"),
            onKeyDown: (event: KeyboardEvent & { readonly currentTarget: HTMLElement }) => {
                // open the submenu of an enabled trigger and focus its first item
                if (isEnabled() && opens(event.key)) {
                    event.preventDefault();
                    event.stopPropagation();
                    control.open("first");
                }
            },
            get children() {
                return (
                    <>
                        {properties.children}
                        <span {...style.attrs(styles.chevron)}>
                            <Icon icon={locale.direction === "rtl" ? caretLeft : caretRight} />
                        </span>
                    </>
                );
            },
        }),
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
        control.close(true, new OpenChange(event.key === "Escape" ? "escape-key" : "none", event));
    } else if (event.key === "Tab") {
        control.root().close(false, new OpenChange("focus-out", event));
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

/** Click an item on Enter and Space, as buttons do. */
function activate(event: PartEvent<KeyboardEvent>): void {
    if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        event.currentTarget.click();
    }
}

/** Focus an element, else what takes the focus in it first. */
function focusIn(element: HTMLElement | undefined): void {
    // go in where the element itself takes no focus, such as what a context menu opens from
    element?.focus();
    if (element !== undefined && element !== element.ownerDocument.activeElement) {
        const inside = element.querySelector<HTMLElement>("button, a[href], [tabindex]");
        (tabbablesIn(element)[0] ?? inside)?.focus();
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
        control.root().close(true, new OpenChange("item-press", event));
    }
}

/** The properties of a menu's action. */
export interface MenuActionProperties<Inputs extends ActionInputs = ActionInputs> {
    /** The action. */
    readonly action: Action<Inputs>;
    /** Where it runs, where the menu opened by default. */
    readonly element?: Element | null;
    /** The values the item gives, the rest from where it runs. */
    readonly given?: Partial<ValuesOf<Inputs>>;
    /** The icon before the title. */
    readonly icon?: IconBodies;
    /** The title, the action's by default. */
    readonly children?: JSX.Element;
}

/** Render an action as the menu's item with its keys, left out where it cannot run. */
export function MenuAction<Inputs extends ActionInputs>(
    properties: MenuActionProperties<Inputs>,
): JSX.Element {
    // run it where the menu opened, by the keys the person has for it
    const actions = useActions();
    const locale = useLocale();
    const isCommand = useCommandPlatform();
    const control = useMenu();
    const element = () =>
        properties.element === undefined ? control.origin() : properties.element;
    const keys = () => actions.keybindings(properties.action, isCommand())[0];
    const own = useAction(() => ({
        action: properties.action,
        element: element(),
        given: properties.given,
        isFollowed: control.root().isOpen(),
    }));
    const title = () => (
        <>
            <Show when={properties.icon}>{(icon) => <Icon icon={icon()} />}</Show>
            {contentOr(properties, () =>
                actions.sentence(properties.action, element(), locale, properties.given),
            )}
        </>
    );
    const shortcut = () => (
        <Show when={properties.given === undefined ? keys() : undefined}>
            {(pressed) => <MenuShortcut keys={pressed()} />}
        </Show>
    );

    return (
        <Show when={own.isEnabled()}>
            <Show
                when={own.input()}
                fallback={
                    <Show
                        when={own.isOn() !== undefined}
                        fallback={
                            <MenuItem
                                variant={
                                    properties.action.isDestructive ? "destructive" : "default"
                                }
                                onSelect={() => own.start()}
                            >
                                {title()}
                                {shortcut()}
                            </MenuItem>
                        }
                    >
                        <MenuCheckboxItem
                            checked={own.isOn() === true}
                            onCheckedChange={() => own.start()}
                        >
                            {title()}
                            {shortcut()}
                        </MenuCheckboxItem>
                    </Show>
                }
            >
                <Menu>
                    <MenuSubTrigger>
                        {title()}
                        {shortcut()}
                    </MenuSubTrigger>
                    <MenuContent side="right" align="start">
                        <MenuChoices state={own} isRepeated={properties.action.isRepeated} />
                    </MenuContent>
                </Menu>
            </Show>
        </Show>
    );
}

/** Render the choices of an action's input as a submenu's items, marking those it has. */
function MenuChoices(properties: {
    /** The action's state. */
    readonly state: ActionState;
    /** Whether a person picks several in a row, the menu staying open. */
    readonly isRepeated: boolean;
}): JSX.Element {
    // read whether some are marked, the others then rendered alike
    const own = untrack(() => properties.state);
    const isMarked = () => properties.isRepeated || own.chosen().length > 0;

    return (
        <For each={own.choices()}>
            {(choice) => {
                const content = () => (
                    <>
                        <ChoiceIcon choice={choice} />
                        {choice.title}
                    </>
                );

                return (
                    <Show
                        when={isMarked()}
                        fallback={
                            <MenuItem onSelect={() => own.start(choice.value)}>
                                {content()}
                            </MenuItem>
                        }
                    >
                        <MenuCheckboxItem
                            checked={own.isChosen(choice)}
                            onCheckedChange={() => own.start(choice.value)}
                            onSelect={(event) => {
                                // leave the menu open for another
                                if (properties.isRepeated) {
                                    event.preventDefault();
                                }
                            }}
                        >
                            {content()}
                        </MenuCheckboxItem>
                    </Show>
                );
            }}
        </For>
    );
}
