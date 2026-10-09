import { Icon } from "@destack/icon";
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
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createSignal,
    createUniqueId,
    type JSX,
    merge,
    omit,
    onCleanup,
    Portal,
    type Setter,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import { type PartAttributes, type Render, rendered, renderPart } from "../part/index.ts";
import { ListState } from "../focus/index.ts";

/** The selector of the links and triggers at the top level of a navigation menu. */
const TOP = "[data-navigation-top]";

/** The selector of the elements that take the focus with Tab. */
const TABBABLE =
    "a[href], button:not(:disabled), input:not(:disabled), [tabindex]:not([tabindex='-1'])";

/** The anchor name of the open trigger, scoped to each navigation menu, which the indicator points at. */
const ACTIVE_ANCHOR = "--destack-navigation-menu-active";

/** The wait before a resting pointer opens a panel, in milliseconds. */
const DELAY_DURATION = 200;

/** The time after the pointer leaves a trigger within which the next one opens at once, in milliseconds. */
const SKIP_DELAY_DURATION = 300;

/** The navigation menu around an element, null outside one. */
const NavigationMenuContext = createContext<NavigationMenuControl | null>(null);

/** Whether the elements around render inside a navigation menu's content. */
const NavigationMenuContentContext = createContext(false);

/** The value of the nearest navigation menu item, null outside one. */
const NavigationMenuItemContext = createContext<string | null>(null);

/** The styles of a navigation menu and its elements. */
const styles = style.create({
    menu: {
        position: "relative",
        display: "flex",
        alignItems: "center",
    },
    list: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
    },
    item: {
        position: "relative",
    },
    trigger: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        gap: space[1],
        height: size[3],
        paddingInline: space[4],
        borderRadius: radius[3],
        backgroundColor: {
            default: color.background,
            ":hover": { default: null, [media.hover]: color.accent },
            ":focus-visible": color.accent,
        },
        color: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.accentForeground },
        },
        fontWeight: weight.medium,
        cursor: "pointer",
        transitionProperty: "color, background-color",
        transitionDuration: motion.durationShort,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
    open: {
        backgroundColor: `color-mix(in oklab, ${color.accent} 50%, transparent)`,
    },
    turned: {
        transform: "rotate(180deg)",
    },
    chevron: {
        display: "inline-flex",
        transform: "none",
        transitionProperty: "transform",
        transitionDuration: motion.durationShort,
    },
    menuScope: {
        anchorScope: ACTIVE_ANCHOR,
    },
    anchor: {
        anchorName: ACTIVE_ANCHOR,
    },
    surface: {
        position: "absolute",
        top: "100%",
        insetInlineStart: 0,
        zIndex: 1,
        marginTop: space[2],
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.popover,
        color: color.popoverForeground,
        boxShadow: shadow.overlay,
    },
    content: {
        padding: space[2],
    },
    indicator: {
        position: "absolute",
        positionAnchor: ACTIVE_ANCHOR,
        positionArea: "block-end",
        zIndex: 2,
        width: space[2],
        height: space[2],
        marginTop: space[1],
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        borderLeftStyle: "solid",
        borderLeftWidth: stroke.border,
        borderColor: color.border,
        backgroundColor: color.popover,
        transform: "rotate(45deg)",
    },
    link: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
        padding: space[2],
        borderRadius: radius[2],
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.accent },
            ":focus-visible": color.accent,
        },
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
});

/** The open item, hover timing and viewport of a navigation menu, which its triggers and contents share. */
export class NavigationMenuControl {
    /** The value of the open item, undefined when every item is closed. */
    readonly open: Accessor<string | undefined>;
    /** The element open panels render into, undefined without a viewport. */
    readonly viewport: Accessor<HTMLElement | undefined>;
    /** The top-level links and triggers in document order, which left and right move between. */
    readonly list: ListState;
    /** The properties of the root, read for its hover delays. */
    readonly #properties: NavigationMenuProperties;
    /** Replace the open item and tell the change handler. */
    readonly #setOpen: (item: string | undefined) => void;
    /** The prefix of the ids of the items' triggers and contents. */
    readonly #prefix: string;
    /** Replace the viewport element. */
    readonly #setViewport: Setter<HTMLElement | undefined>;
    /** The pending open of a hovered trigger. */
    #timer: ReturnType<typeof setTimeout> | undefined;
    /** The time the pointer last left a trigger, in milliseconds since the page loaded. */
    #leftAt: number;

    /** Create a navigation menu with its open item, cancelling a pending open when its owner disposes. */
    constructor(properties: NavigationMenuProperties) {
        // follow the controlled open item or the menu's own
        const [open, setOpen] = createControllableSignal<string | undefined>({
            isControlled: () => "value" in properties,
            value: () => properties.value,
            defaultValue: properties.defaultValue,
            onChange: (next) => properties.onValueChange?.(next),
        });
        const [viewport, setViewport] = createSignal<HTMLElement | undefined>(undefined, {
            ownedWrite: true,
        });
        this.open = open;
        this.viewport = viewport;
        this.list = new ListState({
            orientation: "horizontal",
            isLooping: true,
            isTypeahead: false,
        });
        this.#properties = properties;
        this.#setOpen = setOpen;
        this.#setViewport = setViewport;
        this.#prefix = createUniqueId();
        this.#timer = undefined;
        this.#leftAt = Number.NEGATIVE_INFINITY;
        onCleanup(() => clearTimeout(this.#timer));
    }

    /** Open an item now, closing the one open before. */
    show(item: string): void {
        clearTimeout(this.#timer);
        this.#setOpen(item);
    }

    /** Close every item. */
    hide(): void {
        clearTimeout(this.#timer);
        this.#setOpen(undefined);
    }

    /** Open a hovered item after the delay, or at once while another is open or the pointer just left one. */
    enter(item: string): void {
        // skip the delay while a panel is open or right after one closed
        const skip = this.#properties.skipDelayDuration ?? SKIP_DELAY_DURATION;
        const isSkipping = this.open() !== undefined || performance.now() - this.#leftAt < skip;
        clearTimeout(this.#timer);
        if (isSkipping) {
            this.show(item);
        } else {
            this.#timer = setTimeout(
                () => this.show(item),
                this.#properties.delayDuration ?? DELAY_DURATION,
            );
        }
    }

    /** Cancel a pending open and remember when the pointer left a trigger. */
    leave(): void {
        clearTimeout(this.#timer);
        this.#leftAt = performance.now();
    }

    /** Set the element open panels render into. */
    setViewport(element: HTMLElement): void {
        this.#setViewport(element);
    }

    /** Add a top-level link or trigger until it unmounts, giving its element the focus as the arrow keys reach it. */
    join(key: string, element: () => HTMLElement | undefined): void {
        this.list.add({ key, text: () => key, isDisabled: () => false, element });
    }

    /** Move the focus to the top-level entry after one, none past the last. */
    enterAfter(key: string): boolean {
        // read the entry after the key
        const index = this.list.collection.index(key);
        const next = index === undefined ? undefined : this.list.collection.at(index + 1);
        if (next === undefined) {
            return false;
        }
        this.list.focus.enter(next);

        return true;
    }

    /** Build the id of an item's content. */
    contentId(item: string): string {
        return `${this.#prefix}-${item.replaceAll(/\s/gu, "-")}-content`;
    }

    /** Build the id of an item's trigger. */
    triggerId(item: string): string {
        return `${this.#prefix}-${item.replaceAll(/\s/gu, "-")}-trigger`;
    }
}

/** The properties of an element of a navigation menu, the native element's attributes included. */
export type NavigationMenuElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** The properties of a navigation menu. */
export type NavigationMenuProperties = NavigationMenuElementProperties<
    Omit<JSX.HTMLAttributes<HTMLElement>, "onFocusOut" | "onKeyDown" | "onPointerLeave">
> & {
    /** The value of the open item, which makes the open item controlled, undefined while every item is closed. */
    readonly value?: string | undefined;
    /** The value of the item open at first while uncontrolled, every item closed by default. */
    readonly defaultValue?: string;
    /** Handle an item opening, or every item closing with undefined. */
    readonly onValueChange?: (value: string | undefined) => void;
    /** Whether open panels render into one viewport below the list, true by default. */
    readonly viewport?: boolean;
    /** The wait before a resting pointer opens a panel in milliseconds, 200 by default. */
    readonly delayDuration?: number;
    /** The time after leaving a trigger within which the next opens at once in milliseconds, 300 by default. */
    readonly skipDelayDuration?: number;
};

/** Return the StyleX styles of a navigation menu's trigger, for its top-level links. */
export function navigationMenuTriggerStyle(): style.Styles {
    return [text.footnote, styles.trigger];
}

/** Read the navigation menu around an element, refusing elements outside one. */
function useNavigationMenu(): NavigationMenuControl {
    const control = useContext(NavigationMenuContext);
    if (control === null) {
        throw new TypeError("navigation menu elements need a navigation menu around them");
    }

    return control;
}

/** Read the item around an element, refusing elements outside one. */
function useItem(): string {
    const item = useContext(NavigationMenuItemContext);
    if (item === null) {
        throw new TypeError("a navigation menu trigger or content needs an item around it");
    }

    return item;
}

/** Render a site's navigation, whose items disclose panels of links and close once the focus or pointer leaves. */
export function NavigationMenu(properties: NavigationMenuProperties): JSX.Element {
    const control = new NavigationMenuControl(properties);
    const rest = omit(
        properties,
        "value",
        "defaultValue",
        "onValueChange",
        "viewport",
        "delayDuration",
        "skipDelayDuration",
        "xstyle",
        "style",
        "children",
    );

    return (
        <NavigationMenuContext value={control}>
            <nav
                data-slot="navigation-menu"
                data-viewport={properties.viewport === false ? "false" : "true"}
                {...rest}
                onFocusOut={(event) => {
                    // close once the focus leaves the navigation
                    if (
                        !(event.relatedTarget instanceof Node) ||
                        !event.currentTarget.contains(event.relatedTarget)
                    ) {
                        control.hide();
                    }
                }}
                onPointerLeave={() => control.hide()}
                onKeyDown={(event) => {
                    // close on Escape and return the focus to the open item's trigger
                    const item = control.open();
                    if (event.key === "Escape" && item !== undefined) {
                        control.hide();
                        document.getElementById(control.triggerId(item))?.focus();
                    }
                }}
                {...style.attributes(
                    [styles.menu, styles.menuScope, properties.xstyle],
                    properties.style,
                )}
            >
                {properties.children}
                <Show when={properties.viewport !== false}>
                    <NavigationMenuViewport />
                </Show>
            </nav>
        </NavigationMenuContext>
    );
}

/** Render the list of top-level links and triggers, which left and right arrow keys move between. */
export function NavigationMenuList(
    properties: NavigationMenuElementProperties<
        Omit<JSX.HTMLAttributes<HTMLUListElement>, "onKeyDown" | "onFocusOut">
    >,
): JSX.Element {
    // move between the top-level entries in the reading direction
    const control = useNavigationMenu();
    const locale = useLocale();

    return renderPart("ul", "navigation-menu-list", properties, styles.list, {
        onKeyDown: (event) => {
            // move between the top-level links and triggers
            if (event.target instanceof Element && event.target.matches(TOP)) {
                control.list.focus.move(event, locale.direction);
            }
        },
        onFocusOut: (event) => control.list.focus.focusOut(event),
    });
}

/** The properties of a navigation menu's item, the native element's attributes included. */
export type NavigationMenuItemProperties = NavigationMenuElementProperties<
    Omit<JSX.LiHTMLAttributes<HTMLLIElement>, "value">
> & {
    /** The value that opens the item's panel, a generated one by default. */
    readonly value?: string;
};

/** Render one top-level entry of a navigation menu. */
export function NavigationMenuItem(properties: NavigationMenuItemProperties): JSX.Element {
    const item = properties.value ?? createUniqueId();
    const rest = omit(properties, "value", "xstyle", "style");

    return (
        <NavigationMenuItemContext value={item}>
            <li
                data-slot="navigation-menu-item"
                {...rest}
                {...style.attributes([styles.item, properties.xstyle], properties.style)}
            />
        </NavigationMenuItemContext>
    );
}

/** Render the button that discloses its item's panel on click, or on hover after the delay. */
export function NavigationMenuTrigger(
    properties: NavigationMenuElementProperties<
        Omit<
            JSX.ButtonHTMLAttributes<HTMLButtonElement>,
            "onClick" | "onPointerEnter" | "onPointerLeave" | "onKeyDown" | "onFocus" | "ref"
        >
    >,
): JSX.Element {
    // read the menu and the item the trigger opens
    const control = useNavigationMenu();
    const item = useItem();
    const rest = omit(properties, "xstyle", "style", "children");
    const isOpen = (): boolean => control.open() === item;
    let element: HTMLButtonElement | undefined;
    control.join(item, () => element);

    return (
        <button
            type="button"
            id={control.triggerId(item)}
            aria-expanded={isOpen() ? "true" : "false"}
            aria-controls={control.contentId(item)}
            data-navigation-top=""
            data-slot="navigation-menu-trigger"
            data-state={isOpen() ? "open" : "closed"}
            {...rest}
            ref={(button) => (element = button)}
            onFocus={() => control.list.focus.focusIn(item)}
            onClick={() => (isOpen() ? control.hide() : control.show(item))}
            onPointerEnter={() => control.enter(item)}
            onPointerLeave={() => control.leave()}
            onKeyDown={(event) => {
                // carry Tab from an open trigger into its panel, wherever the panel renders
                const first = document
                    .getElementById(control.contentId(item))
                    ?.querySelector<HTMLElement>(TABBABLE);
                if (
                    event.key === "Tab" &&
                    !event.shiftKey &&
                    isOpen() &&
                    first !== null &&
                    first !== undefined
                ) {
                    event.preventDefault();
                    first.focus();
                }
            }}
            {...style.attributes(
                [
                    text.footnote,
                    styles.trigger,
                    isOpen() && styles.open,
                    isOpen() && styles.anchor,
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            {properties.children}
            <span {...style.attrs(styles.chevron, isOpen() && styles.turned)}>
                <Icon name="caret-down" />
            </span>
        </button>
    );
}

/** Render the panel of links its item's trigger discloses, inside the viewport when the menu has one. */
export function NavigationMenuContent(
    properties: NavigationMenuElementProperties<
        Omit<JSX.HTMLAttributes<HTMLDivElement>, "onKeyDown">
    >,
): JSX.Element {
    // read the menu and the item the panel belongs to
    const control = useNavigationMenu();
    const item = useItem();
    const rest = omit(properties, "xstyle", "style", "children");

    // build the panel, framed on its own without a viewport
    const panel = (isFramed: boolean) => (
        <div
            id={control.contentId(item)}
            hidden={control.open() !== item}
            data-slot="navigation-menu-content"
            {...rest}
            onKeyDown={(event) => leavePanel(event, control, item)}
            {...style.attributes(
                [isFramed && styles.surface, styles.content, properties.xstyle],
                properties.style,
            )}
        >
            <NavigationMenuContentContext value={true}>
                {properties.children}
            </NavigationMenuContentContext>
        </div>
    );

    return (
        <Show when={control.viewport()} fallback={panel(true)}>
            {(viewport) => <Portal mount={viewport()}>{panel(false)}</Portal>}
        </Show>
    );
}

/** Render the frame below the list that the open panel shows in, hidden while every panel is closed. */
export function NavigationMenuViewport(
    properties: NavigationMenuElementProperties<Omit<JSX.HTMLAttributes<HTMLDivElement>, "ref">>,
): JSX.Element {
    const control = useNavigationMenu();

    return renderPart("div", "navigation-menu-viewport", properties, styles.surface, {
        get hidden() {
            return control.open() === undefined;
        },
        ref: (element) => control.setViewport(element),
    });
}

/** Render the arrow under the open item's trigger, through CSS anchor positioning. */
export function NavigationMenuIndicator(
    properties: NavigationMenuElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useNavigationMenu();

    return renderPart("div", "navigation-menu-indicator", properties, styles.indicator, {
        "aria-hidden": "true",
        get hidden() {
            return control.open() === undefined;
        },
    });
}

/** The properties of a link of a navigation menu, the native anchor's attributes included. */
export type NavigationMenuLinkProperties = NavigationMenuElementProperties<
    JSX.AnchorHTMLAttributes<HTMLAnchorElement>
> & {
    /** Whether the link points at the current page. */
    readonly active?: boolean;
    /** Render another element with the link's attributes, such as a router's link. */
    readonly render?: Render;
};

/** Render a link of a navigation menu, marked as the current page when active. */
export function NavigationMenuLink(properties: NavigationMenuLinkProperties): JSX.Element {
    // join a top-level link to the menu's entries, and mark it and the current page over its styles
    const menu = useContext(NavigationMenuContext);
    const isTop = !useContext(NavigationMenuContentContext);
    const key = createUniqueId();
    let element: HTMLElement | undefined;
    if (isTop && menu !== null) {
        menu.join(key, () => element);
    }
    const rest = omit(properties, "active", "xstyle", "style", "render");
    const part: PartAttributes = merge(
        {
            get "aria-current"() {
                return properties.active === true ? "page" : undefined;
            },
            get "data-active"() {
                return properties.active === true ? "true" : undefined;
            },
            "data-navigation-top": isTop ? "" : undefined,
            "data-slot": "navigation-menu-link",
            ref: (link: HTMLElement) => {
                element = link;
            },
            onFocus: () => {
                // follow the focus onto a top-level link
                if (isTop) {
                    menu?.list.focus.focusIn(key);
                }
            },
        },
        () => style.attributes([text.footnote, styles.link, properties.xstyle], properties.style),
    );

    return rendered(properties.render, part, rest, () => <a {...part} {...rest} />);
}

/** Move Tab past a panel's ends to its trigger and to the next top-level entry, as if the panel followed its trigger. */
function leavePanel(
    event: KeyboardEvent & { readonly currentTarget: HTMLDivElement },
    control: NavigationMenuControl,
    item: string,
): void {
    // read the panel's ends and the trigger it belongs to
    const tabbable = [...event.currentTarget.querySelectorAll<HTMLElement>(TABBABLE)];
    const trigger = document.getElementById(control.triggerId(item));
    if (event.key !== "Tab" || trigger === null) {
        return;
    }

    // go back to the trigger from the first element, and on to the next entry from the last
    if (event.shiftKey && document.activeElement === tabbable[0]) {
        event.preventDefault();
        trigger.focus();
    } else if (!event.shiftKey && document.activeElement === tabbable.at(-1)) {
        if (control.enterAfter(item)) {
            event.preventDefault();
        }
    }
}
