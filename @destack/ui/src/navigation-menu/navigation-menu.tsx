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
import { type PartAttributes, type Render, rendered } from "../part/index.ts";
import { itemsOf, moveFocus } from "../focus/index.ts";

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

/** The id of the nearest navigation menu item, null outside one. */
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
        margin: 0,
        padding: 0,
        listStyle: "none",
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
        borderWidth: 0,
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
        textDecoration: "none",
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
        borderStyle: "solid",
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
        color: "inherit",
        textDecoration: "none",
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
    /** The id of the open item, undefined when every item is closed. */
    readonly open: Accessor<string | undefined>;
    /** The element open panels render into, undefined without a viewport. */
    readonly viewport: Accessor<HTMLElement | undefined>;
    /** The properties of the root, read for its hover delays. */
    readonly #properties: NavigationMenuProperties;
    /** Replace the open item. */
    readonly #setOpen: Setter<string | undefined>;
    /** Replace the viewport element. */
    readonly #setViewport: Setter<HTMLElement | undefined>;
    /** The pending open of a hovered trigger. */
    #timer: ReturnType<typeof setTimeout> | undefined;
    /** The time the pointer last left a trigger, in milliseconds since the page loaded. */
    #leftAt: number;

    /** Create a navigation menu with every item closed, cancelling a pending open when its owner disposes. */
    constructor(properties: NavigationMenuProperties) {
        // start closed without a viewport or pending open
        const [open, setOpen] = createSignal<string | undefined>(undefined);
        const [viewport, setViewport] = createSignal<HTMLElement | undefined>(undefined, {
            ownedWrite: true,
        });
        this.open = open;
        this.viewport = viewport;
        this.#properties = properties;
        this.#setOpen = setOpen;
        this.#setViewport = setViewport;
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

    /** Name the id of an item's content. */
    contentId(item: string): string {
        return `${item}-content`;
    }

    /** Name the id of an item's trigger. */
    triggerId(item: string): string {
        return `${item}-trigger`;
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
        Omit<JSX.HTMLAttributes<HTMLUListElement>, "onKeyDown">
    >,
): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style");

    return (
        <ul
            data-slot="navigation-menu-list"
            {...rest}
            onKeyDown={(event) => {
                // move between the top-level links and triggers
                if (event.target instanceof Element && event.target.matches(TOP)) {
                    moveFocus(
                        event,
                        itemsOf(event.currentTarget, TOP),
                        "horizontal",
                        locale.direction,
                    );
                }
            }}
            {...style.attributes([styles.list, properties.xstyle], properties.style)}
        />
    );
}

/** Render one top-level entry of a navigation menu. */
export function NavigationMenuItem(
    properties: NavigationMenuElementProperties<JSX.LiHTMLAttributes<HTMLLIElement>>,
): JSX.Element {
    const item = createUniqueId();
    const rest = omit(properties, "xstyle", "style");

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
            "onClick" | "onPointerEnter" | "onPointerLeave" | "onKeyDown"
        >
    >,
): JSX.Element {
    // read the menu and the item the trigger opens
    const control = useNavigationMenu();
    const item = useItem();
    const rest = omit(properties, "xstyle", "style", "children");
    const isOpen = (): boolean => control.open() === item;

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
            onKeyDown={(event) => leavePanel(event, control.triggerId(item))}
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
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            hidden={control.open() === undefined}
            data-slot="navigation-menu-viewport"
            {...rest}
            ref={(element) => control.setViewport(element)}
            {...style.attributes([styles.surface, properties.xstyle], properties.style)}
        />
    );
}

/** Render the arrow under the open item's trigger, through CSS anchor positioning. */
export function NavigationMenuIndicator(
    properties: NavigationMenuElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useNavigationMenu();
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            aria-hidden="true"
            hidden={control.open() === undefined}
            data-slot="navigation-menu-indicator"
            {...rest}
            {...style.attributes([styles.indicator, properties.xstyle], properties.style)}
        />
    );
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
    // mark a top-level link and the current page over its styles
    const isTop = !useContext(NavigationMenuContentContext);
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
        },
        () => style.attributes([text.footnote, styles.link, properties.xstyle], properties.style),
    );

    return rendered(properties.render, part, rest, () => <a {...part} {...rest} />);
}

/** Move Tab past a panel's ends to its trigger and to the next top-level entry, as if the panel followed its trigger. */
function leavePanel(
    event: KeyboardEvent & { readonly currentTarget: HTMLDivElement },
    triggerId: string,
): void {
    // read the panel's ends and the trigger it belongs to
    const tabbable = [...event.currentTarget.querySelectorAll<HTMLElement>(TABBABLE)];
    const trigger = document.getElementById(triggerId);
    if (event.key !== "Tab" || trigger === null) {
        return;
    }

    // go back to the trigger from the first element, and on to the next entry from the last
    if (event.shiftKey && document.activeElement === tabbable[0]) {
        event.preventDefault();
        trigger.focus();
    } else if (!event.shiftKey && document.activeElement === tabbable.at(-1)) {
        const list = trigger.closest("[data-slot=navigation-menu-list]");
        const entries = list === null ? [] : itemsOf(list, TOP);
        const next = entries[entries.indexOf(trigger) + 1];
        if (next !== undefined) {
            event.preventDefault();
            next.focus();
        }
    }
}
