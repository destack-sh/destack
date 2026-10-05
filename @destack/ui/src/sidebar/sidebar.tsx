import { Icon } from "@destack/icon";
import { t } from "@destack/locale";
import * as style from "@destack/style";
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
import { useLocale } from "@destack/locale/solid";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import {
    createContext,
    createSignal,
    createUniqueId,
    merge,
    omit,
    onSettled,
    Show,
    useContext,
    type Accessor,
} from "solid-js";
import { Button, type ButtonProperties } from "../button/index.ts";
import { Sheet, SheetContent, SheetTitle } from "../sheet/index.ts";
import { Skeleton } from "../skeleton/index.ts";
import { Tooltip, TooltipContent, TooltipContext } from "../tooltip/index.ts";

/** The width of an expanded sidebar, shadcn/ui's 16rem. */
const SIDEBAR_WIDTH = "16rem";

/** The width of a sidebar collapsed to its icons, shadcn/ui's 3rem. */
const SIDEBAR_WIDTH_ICON = "3rem";

/** The key the open state is remembered under in the viewer's local storage. */
const STORAGE_KEY = "destack-sidebar-open";

/** The viewports narrower than shadcn/ui's 768 pixel breakpoint, where the sidebar opens as a sheet. */
const MOBILE_QUERY = "(max-width: 47.999rem)";

/** The look and collapse of a sidebar that sets neither. */
const DEFAULTS: Required<Pick<SidebarProperties, "side" | "variant" | "collapsible">> = {
    side: "left",
    variant: "sidebar",
    collapsible: "offcanvas",
};

/** The sidebar state of the nearest provider, null outside one. */
const SidebarContext = createContext<SidebarControl | null>(null);

/** The styles of a sidebar and its elements. */
const styles = style.create({
    wrapper: {
        display: "flex",
        minHeight: "100vh",
        width: "100%",
    },
    sidebar: {
        position: "sticky",
        top: 0,
        display: "flex",
        flexDirection: "column",
        flexShrink: 0,
        height: "100vh",
        overflow: "hidden",
        backgroundColor: color.card,
        color: color.cardForeground,
        transitionProperty: "width",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
    },
    left: {
        borderRightStyle: "solid",
        borderRightWidth: stroke.border,
        borderRightColor: color.border,
    },
    right: {
        order: 1,
        borderLeftStyle: "solid",
        borderLeftWidth: stroke.border,
        borderLeftColor: color.border,
    },
    floating: {
        height: `calc(100vh - 2 * ${space[2]})`,
        margin: space[2],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[4],
        boxShadow: shadow.raised,
    },
    inner: {
        display: "flex",
        flexDirection: "column",
        width: SIDEBAR_WIDTH,
        height: "100%",
    },
    expanded: { width: SIDEBAR_WIDTH },
    offcanvas: { width: 0, borderWidth: 0 },
    icon: { width: SIDEBAR_WIDTH_ICON },
    sheet: { width: SIDEBAR_WIDTH, padding: 0 },
    hidden: {
        position: "absolute",
        width: stroke.border,
        height: stroke.border,
        overflow: "hidden",
        clipPath: "inset(50%)",
    },
    rail: {
        position: "absolute",
        top: 0,
        bottom: 0,
        width: space[4],
        padding: 0,
        borderWidth: 0,
        backgroundColor: "transparent",
        cursor: "ew-resize",
    },
    inset: {
        display: "flex",
        flex: 1,
        flexDirection: "column",
        minWidth: 0,
        backgroundColor: color.background,
    },
    header: { display: "flex", flexDirection: "column", gap: space[2], padding: space[2] },
    footer: { display: "flex", flexDirection: "column", gap: space[2], padding: space[2] },
    content: {
        display: "flex",
        flex: 1,
        flexDirection: "column",
        gap: space[2],
        minHeight: 0,
        overflow: "auto",
    },
    separator: {
        height: 0,
        marginInline: space[2],
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        borderTopColor: color.border,
    },
    group: { position: "relative", display: "flex", flexDirection: "column", padding: space[2] },
    groupLabel: {
        display: "flex",
        alignItems: "center",
        height: size[2],
        paddingInline: space[2],
        color: color.mutedForeground,
        fontWeight: weight.medium,
        whiteSpace: "nowrap",
        transitionProperty: "opacity",
        transitionDuration: motion.durationMedium,
    },
    fade: { opacity: 0 },
    groupAction: { position: "absolute", top: space[3], right: space[3] },
    menu: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
        margin: 0,
        padding: 0,
        listStyle: "none",
    },
    menuItem: { position: "relative" },
    menuButton: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        width: "100%",
        overflow: "hidden",
        paddingInline: space[2],
        borderWidth: 0,
        borderRadius: radius[3],
        backgroundColor: { default: "transparent", ":hover": color.accent },
        color: { default: "inherit", ":hover": color.accentForeground },
        textAlign: "start",
        textDecoration: "none",
        whiteSpace: "nowrap",
        cursor: "pointer",
        fontFamily: "inherit",
        fontSize: "inherit",
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
    active: {
        backgroundColor: color.accent,
        color: color.accentForeground,
        fontWeight: weight.medium,
    },
    menuAction: { position: "absolute", top: space[1], right: space[1] },
    menuBadge: {
        position: "absolute",
        top: space[2],
        right: space[2],
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
        pointerEvents: "none",
    },
    skeleton: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        height: size[2],
        paddingInline: space[2],
    },
    skeletonIcon: {
        width: space[4],
        height: space[4],
        borderRadius: radius[2],
    },
    skeletonText: {
        flex: 1,
    },
    skeletonWidth: (width: string) => ({
        width,
    }),
    skeletonLine: {
        width: "100%",
        height: space[4],
    },
    menuSub: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
        marginInlineStart: space[4],
        paddingInlineStart: space[2],
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: stroke.border,
        borderInlineStartColor: color.border,
        listStyle: "none",
    },
});

/** The height of a menu button in each size. */
const sizes = style.create({
    default: { height: size[2] },
    sm: { height: size[1] },
    lg: { height: size[4] },
});

/** The side of the screen a sidebar sits on. */
export type SidebarSide = "left" | "right";

/** The look of a sidebar: against the edge, floating, or inset around the main content. */
export type SidebarVariant = "sidebar" | "floating" | "inset";

/** How a sidebar collapses: off the screen, down to its icons, or not at all. */
export type SidebarCollapsible = "offcanvas" | "icon" | "none";

/** The open state of a sidebar on wide and on narrow screens, which its elements share. */
export class SidebarControl {
    /** The id of the sidebar element. */
    readonly id: string;
    /** Whether the viewport is narrow enough to open the sidebar as a sheet. */
    readonly isMobile: Accessor<boolean>;
    /** Whether the sheet of a narrow viewport is open. */
    readonly isMobileOpen: Accessor<boolean>;
    /** The properties of the provider, read for its controlled state and change handler. */
    readonly #properties: SidebarProviderProperties;
    /** The open state on wide screens when uncontrolled. */
    readonly #ownOpen: Accessor<boolean>;
    /** Replace the open state on wide screens. */
    readonly #setOpen: (isOpen: boolean) => void;
    /** Replace the open state of the sheet. */
    readonly #setMobileOpen: (isOpen: boolean) => void;
    /** Replace whether the viewport is narrow. */
    readonly #setMobile: (isMobile: boolean) => void;

    /** Create the state of a sidebar, open unless told otherwise on a wide viewport until the browser says more. */
    constructor(properties: SidebarProviderProperties) {
        // start from the defaults a server renders too
        const [ownOpen, setOpen] = createSignal(properties.defaultOpen !== false);
        const [isMobileOpen, setMobileOpen] = createSignal(false);
        const [isMobile, setMobile] = createSignal(false);

        // keep the state
        this.id = createUniqueId();
        this.isMobile = isMobile;
        this.isMobileOpen = isMobileOpen;
        this.#properties = properties;
        this.#ownOpen = ownOpen;
        this.#setOpen = setOpen;
        this.#setMobileOpen = setMobileOpen;
        this.#setMobile = setMobile;
    }

    /** Take the state the viewer left the sidebar in and follow the viewport's width, returning how to stop. */
    follow(): () => void {
        // start as the viewer left it
        const stored = remembered();
        if (stored !== undefined) {
            this.#setOpen(stored);
        }

        // follow the viewport's width
        const query = window.matchMedia(MOBILE_QUERY);
        const follow = (event: MediaQueryListEvent) => this.#setMobile(event.matches);
        this.#setMobile(query.matches);
        query.addEventListener("change", follow);

        return () => query.removeEventListener("change", follow);
    }

    /** Read whether the sidebar is open on wide screens, controlled or the provider's own. */
    isOpen(): boolean {
        return this.#properties.open ?? this.#ownOpen();
    }

    /** Open or close the sidebar on wide screens and tell the change handler. */
    setOpen(isOpen: boolean): void {
        this.#setOpen(isOpen);
        this.#properties.onOpenChange?.(isOpen);
        remember(isOpen);
    }

    /** Open or close the sheet of a narrow viewport. */
    setMobileOpen(isOpen: boolean): void {
        this.#setMobileOpen(isOpen);
    }

    /** Toggle the sheet on narrow screens and the sidebar on wide ones. */
    toggle(): void {
        if (this.isMobile()) {
            this.#setMobileOpen(!this.isMobileOpen());
        } else {
            this.setOpen(!this.isOpen());
        }
    }
}

/** The properties of a sidebar provider, the native element's attributes included. */
export interface SidebarProviderProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style"
> {
    /** Whether the sidebar is open, which makes the state controlled. */
    readonly open?: boolean;
    /** Whether the sidebar starts open when uncontrolled, true by default. */
    readonly defaultOpen?: boolean;
    /** Handle the sidebar opening or closing on wide screens. */
    readonly onOpenChange?: (open: boolean) => void;
    /** The StyleX styles applied after the wrapper's styles. */
    readonly style?: style.Styles;
}

/** The properties of a sidebar, the native element's attributes included. */
export interface SidebarProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style" | "id"
> {
    /** The side of the screen, left by default. */
    readonly side?: SidebarSide;
    /** The look, against the edge by default. */
    readonly variant?: SidebarVariant;
    /** How it collapses, off the screen by default. */
    readonly collapsible?: SidebarCollapsible;
    /** The StyleX styles applied after the sidebar's styles. */
    readonly style?: style.Styles;
}

/** The properties of an element of a sidebar, the native element's attributes included. */
export type SidebarElementProperties<Attributes> = Omit<Attributes, "class" | "style"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
};

/** The properties of a sidebar menu button, a link with `href` and a button otherwise. */
export type SidebarMenuButtonProperties = SidebarElementProperties<
    JSX.ButtonHTMLAttributes<HTMLButtonElement> & JSX.AnchorHTMLAttributes<HTMLAnchorElement>
> & {
    /** The link target, which renders the button as a link. */
    readonly href?: string;
    /** Whether the button stands for the current page. */
    readonly isActive?: boolean;
    /** The height of the button, default by default. */
    readonly size?: "default" | "sm" | "lg";
    /** The name a tooltip shows while the sidebar is collapsed to its icons. */
    readonly tooltip?: string;
};

/** Read the sidebar state of the nearest provider, refusing elements outside one. */
export function useSidebar(): SidebarControl {
    const control = useContext(SidebarContext);
    if (control === null) {
        throw new TypeError("sidebar elements need a sidebar provider around them");
    }

    return control;
}

/** Hold the open state of a page's sidebar, toggled by its triggers and by Ctrl or Command with B. */
export function SidebarProvider(properties: SidebarProviderProperties): JSX.Element {
    // share one state with the sidebar
    const control = new SidebarControl(properties);
    const rest = omit(properties, "open", "defaultOpen", "onOpenChange", "style");

    // follow the browser once mounted, and toggle the sidebar from the keyboard
    onSettled(() => {
        // follow the remembered state and the viewport's width
        const stop = control.follow();

        // toggle on Ctrl or Command with B
        const toggle = (event: KeyboardEvent) => {
            if (event.key === "b" && (event.metaKey || event.ctrlKey)) {
                event.preventDefault();
                control.toggle();
            }
        };
        document.addEventListener("keydown", toggle);

        return () => {
            stop();
            document.removeEventListener("keydown", toggle);
        };
    });

    return (
        <SidebarContext value={control}>
            <div
                data-slot="sidebar-wrapper"
                {...rest}
                {...style.attrs(styles.wrapper, properties.style)}
            />
        </SidebarContext>
    );
}

/** Render the sidebar: a column beside the content on wide screens and a sheet on narrow ones. */
export function Sidebar(properties: SidebarProperties): JSX.Element {
    // read the state and the look
    const control = useSidebar();
    const locale = useLocale();
    const sidebar = merge(DEFAULTS, properties);
    const rest = omit(sidebar, "side", "variant", "collapsible", "style", "children");
    const state = (): "expanded" | "collapsed" => (control.isOpen() ? "expanded" : "collapsed");
    const width = () =>
        control.isOpen() || sidebar.collapsible === "none"
            ? styles.expanded
            : styles[sidebar.collapsible === "icon" ? "icon" : "offcanvas"];

    return (
        <Show
            when={!control.isMobile()}
            fallback={
                <Sheet
                    open={control.isMobileOpen()}
                    onOpenChange={(isOpen) => control.setMobileOpen(isOpen)}
                >
                    <SheetContent
                        side={sidebar.side}
                        data-slot="sidebar"
                        data-mobile="true"
                        style={styles.sheet}
                    >
                        <SheetTitle style={styles.hidden}>{locale.render(t`Sidebar`)}</SheetTitle>
                        <div {...style.attrs(styles.inner)}>{sidebar.children}</div>
                    </SheetContent>
                </Sheet>
            }
        >
            <div
                id={control.id}
                data-slot="sidebar"
                data-state={state()}
                data-collapsible={state() === "collapsed" ? sidebar.collapsible : undefined}
                data-variant={sidebar.variant}
                data-side={sidebar.side}
                inert={state() === "collapsed" && sidebar.collapsible === "offcanvas"}
                {...rest}
                {...style.attrs(
                    text.footnote,
                    styles.sidebar,
                    styles[sidebar.side],
                    sidebar.variant === "sidebar" ? null : styles.floating,
                    width(),
                    sidebar.style,
                )}
            >
                <div {...style.attrs(styles.inner)}>{sidebar.children}</div>
            </div>
        </Show>
    );
}

/** Render the button that opens and closes the sidebar. */
export function SidebarTrigger(properties: Omit<ButtonProperties, "onClick">): JSX.Element {
    const control = useSidebar();
    const locale = useLocale();

    return (
        <Button
            variant="ghost"
            size="icon-sm"
            aria-label={locale.render(t`Toggle sidebar`)}
            aria-controls={control.id}
            aria-expanded={
                (control.isMobile() ? control.isMobileOpen() : control.isOpen()) ? "true" : "false"
            }
            data-slot="sidebar-trigger"
            {...properties}
            onClick={() => control.toggle()}
        >
            <Icon name="sidebar-simple" />
        </Button>
    );
}

/** Render the strip along the sidebar's inner edge that toggles it on click. */
export function SidebarRail(
    properties: SidebarElementProperties<
        Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "onClick">
    >,
): JSX.Element {
    // read the sidebar and the locale
    const control = useSidebar();
    const locale = useLocale();
    const rest = omit(properties, "style");

    return (
        <button
            type="button"
            tabindex={-1}
            aria-label={locale.render(t`Toggle sidebar`)}
            data-slot="sidebar-rail"
            {...rest}
            onClick={() => control.toggle()}
            {...style.attrs(styles.rail, properties.style)}
        />
    );
}

/** Render the main content beside the sidebar. */
export function SidebarInset(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <main
            data-slot="sidebar-inset"
            {...rest}
            {...style.attrs(styles.inset, properties.style)}
        />
    );
}

/** Render the top of the sidebar. */
export function SidebarHeader(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="sidebar-header"
            {...rest}
            {...style.attrs(styles.header, properties.style)}
        />
    );
}

/** Render the bottom of the sidebar. */
export function SidebarFooter(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="sidebar-footer"
            {...rest}
            {...style.attrs(styles.footer, properties.style)}
        />
    );
}

/** Render the scrolling middle of the sidebar that holds its groups. */
export function SidebarContent(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="sidebar-content"
            {...rest}
            {...style.attrs(styles.content, properties.style)}
        />
    );
}

/** Render a line between parts of the sidebar. */
export function SidebarSeparator(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            role="separator"
            data-slot="sidebar-separator"
            {...rest}
            {...style.attrs(styles.separator, properties.style)}
        />
    );
}

/** Render a group of the sidebar's menus. */
export function SidebarGroup(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            role="group"
            data-slot="sidebar-group"
            {...rest}
            {...style.attrs(styles.group, properties.style)}
        />
    );
}

/** Render the label of a group, faded while the sidebar shows only icons. */
export function SidebarGroupLabel(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useSidebar();
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="sidebar-group-label"
            {...rest}
            {...style.attrs(
                text.caption,
                styles.groupLabel,
                !control.isOpen() && !control.isMobile() && styles.fade,
                properties.style,
            )}
        />
    );
}

/** Render a button in the corner of a group, such as one that adds an entry. */
export function SidebarGroupAction(properties: ButtonProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <Button
            variant="ghost"
            size="icon-xs"
            data-slot="sidebar-group-action"
            {...rest}
            style={[styles.groupAction, properties.style]}
        />
    );
}

/** Render the body of a group. */
export function SidebarGroupContent(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return <div data-slot="sidebar-group-content" {...rest} {...style.attrs(properties.style)} />;
}

/** Render a list of menu entries. */
export function SidebarMenu(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLUListElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <ul data-slot="sidebar-menu" {...rest} {...style.attrs(styles.menu, properties.style)} />
    );
}

/** Render one entry of a menu. */
export function SidebarMenuItem(
    properties: SidebarElementProperties<JSX.LiHTMLAttributes<HTMLLIElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <li
            data-slot="sidebar-menu-item"
            {...rest}
            {...style.attrs(styles.menuItem, properties.style)}
        />
    );
}

/** Render an entry's button, or its link with `href`, with a tooltip naming it while the sidebar shows only icons. */
export function SidebarMenuButton(properties: SidebarMenuButtonProperties): JSX.Element {
    const rest = omit(properties, "tooltip");

    return (
        <Show when={properties.tooltip} fallback={<SidebarMenuButtonElement {...rest} />}>
            {(tooltip) => (
                <Tooltip delayDuration={0}>
                    <SidebarMenuButtonElement {...rest} />
                    <TooltipContent side="right">{tooltip()}</TooltipContent>
                </Tooltip>
            )}
        </Show>
    );
}

/** Render the button or link of an entry, showing the tooltip around it while the sidebar is collapsed. */
function SidebarMenuButtonElement(
    properties: Omit<SidebarMenuButtonProperties, "tooltip">,
): JSX.Element {
    // read the sidebar and the tooltip, which only a collapsed sidebar shows
    const control = useSidebar();
    const tooltip = useContext(TooltipContext);
    const isHinting = (): boolean => tooltip !== null && !control.isOpen() && !control.isMobile();
    const rest = omit(properties, "href", "isActive", "size", "style");
    const hint = () => {
        if (isHinting()) {
            tooltip?.open();
        }
    };
    const shared = () => ({
        "data-slot": "sidebar-menu-button",
        "data-active": properties.isActive === true ? "true" : undefined,
        "aria-describedby": isHinting() ? tooltip?.id : undefined,
        ref: (element: HTMLElement) => tooltip?.setTrigger(element),
        onPointerEnter: () => hint(),
        onPointerLeave: () => tooltip?.close(),
        onFocus: () => hint(),
        onBlur: () => tooltip?.close(),
        ...style.attrs(
            styles.menuButton,
            sizes[properties.size ?? "default"],
            properties.isActive === true && styles.active,
            properties.style,
        ),
    });

    return (
        <Show when={properties.href} fallback={<button type="button" {...rest} {...shared()} />}>
            {(href) => (
                <a
                    href={href()}
                    aria-current={properties.isActive === true ? "page" : undefined}
                    {...rest}
                    {...shared()}
                />
            )}
        </Show>
    );
}

/** Render placeholder rows of a menu while its entries load, through the shared skeleton. */
export function SidebarMenuSkeleton(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>> & {
        /** Whether the row shows a square in place of an icon. */
        readonly showIcon?: boolean;
    },
): JSX.Element {
    // vary the text's width between rows, as shadcn/ui's skeleton does
    const rest = omit(properties, "showIcon", "style");
    const width = `${50 + Math.floor(Math.random() * 40)}%`;

    return (
        <div
            data-slot="sidebar-menu-skeleton"
            {...rest}
            {...style.attrs(styles.skeleton, properties.style)}
        >
            <Show when={properties.showIcon === true}>
                <Skeleton data-sidebar="menu-skeleton-icon" style={styles.skeletonIcon} />
            </Show>
            <div {...style.attrs(styles.skeletonText, styles.skeletonWidth(width))}>
                <Skeleton data-sidebar="menu-skeleton-text" style={styles.skeletonLine} />
            </div>
        </div>
    );
}

/** Read the open state the viewer left the sidebar in, undefined when none is stored or storage is blocked. */
function remembered(): boolean | undefined {
    try {
        const stored = localStorage.getItem(STORAGE_KEY);

        return stored === null ? undefined : stored === "true";
    } catch {
        // start from the default where the browser blocks storage
        return undefined;
    }
}

/** Remember the open state for the viewer's next visit, skipping browsers that block storage. */
function remember(isOpen: boolean): void {
    try {
        localStorage.setItem(STORAGE_KEY, String(isOpen));
    } catch {
        // keep the state for this visit only where the browser blocks storage
        return;
    }
}

/** Render a button at the end of an entry, such as one that opens the entry's menu. */
export function SidebarMenuAction(properties: ButtonProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <Button
            variant="ghost"
            size="icon-xs"
            data-slot="sidebar-menu-action"
            {...rest}
            style={[styles.menuAction, properties.style]}
        />
    );
}

/** Render a count or status at the end of an entry. */
export function SidebarMenuBadge(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="sidebar-menu-badge"
            {...rest}
            {...style.attrs(text.caption, styles.menuBadge, properties.style)}
        />
    );
}

/** Render a nested list of an entry's sub-entries. */
export function SidebarMenuSub(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLUListElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <ul
            data-slot="sidebar-menu-sub"
            {...rest}
            {...style.attrs(styles.menuSub, properties.style)}
        />
    );
}

/** Render one sub-entry. */
export function SidebarMenuSubItem(
    properties: SidebarElementProperties<JSX.LiHTMLAttributes<HTMLLIElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <li
            data-slot="sidebar-menu-sub-item"
            {...rest}
            {...style.attrs(styles.menuItem, properties.style)}
        />
    );
}

/** Render a sub-entry's link, marked as the current page when active. */
export function SidebarMenuSubButton(
    properties: SidebarElementProperties<JSX.AnchorHTMLAttributes<HTMLAnchorElement>> & {
        readonly isActive?: boolean;
    },
): JSX.Element {
    const rest = omit(properties, "isActive", "style");

    return (
        <a
            aria-current={properties.isActive === true ? "page" : undefined}
            data-slot="sidebar-menu-sub-button"
            data-active={properties.isActive === true ? "true" : undefined}
            {...rest}
            {...style.attrs(
                styles.menuButton,
                sizes.sm,
                properties.isActive === true && styles.active,
                properties.style,
            )}
        />
    );
}
