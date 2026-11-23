import { state } from "@destack/theme/state";
import type { Environment } from "@destack/environment";
import { schema } from "@destack/schema";
import { TelemetryScope } from "@destack/telemetry";
import { Icon } from "@destack/ui/icon";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import {
    color,
    motion,
    radius,
    size,
    space,
    stroke,
    weight,
    width,
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
    onSettled,
    type Setter,
    Show,
    useContext,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { StoredItem } from "@destack/view/primitives/storage";
import { Hover, Move } from "../interaction/index.ts";
import {
    type ElementPartProperties,
    type PartAttributes,
    type PartEvent,
    mergeProperties,
    type Render,
    useRender,
    renderPart,
} from "../part/index.ts";
import { Button, type ButtonProperties } from "../button/index.ts";
import { Input, type InputProperties } from "../input/index.ts";
import { Sheet, SheetContent, SheetTitle } from "../sheet/index.ts";
import { Skeleton } from "../skeleton/index.ts";
import { Tooltip, TooltipContent, TooltipContext } from "../tooltip/index.ts";
import { visuallyHiddenStyle } from "../visually-hidden/index.ts";

/** The 32-bit FNV-1a offset basis, which starts each skeleton row's hash. */
const FNV_OFFSET = 2_166_136_261;

/** The 32-bit FNV-1a prime, which mixes each character into the hash. */
const FNV_PRIME = 16_777_619;

/** The key the open state is remembered under in the viewer's local storage. */
const STORAGE_KEY = "destack-sidebar-open";

/** The key the dragged width is remembered under in the viewer's local storage. */
const WIDTH_KEY = "destack-sidebar-width";

/** A width the viewer left the sidebar at, in pixels. */
const StoredWidth = schema.number().positive();

/** The narrowest a dragged sidebar gets in pixels, 13rem, wide enough to read a project's name. */
const MIN_WIDTH = 208;

/** The widest a dragged sidebar gets in pixels, 25rem, which leaves the page most of the screen. */
const MAX_WIDTH = 400;

/** The viewports narrower than 768 pixels, where the sidebar opens as a sheet. */
const MOBILE_QUERY = "(max-width: 47.999rem)";

/** The look and collapse of a sidebar that sets neither. */
const DEFAULTS: Required<Pick<SidebarProperties, "side" | "variant" | "collapsible">> = {
    side: "left",
    variant: "sidebar",
    collapsible: "offcanvas",
};

/** The sidebar state of the nearest provider, null outside one. */
const SidebarContext = createContext<SidebarControl | null>(null);

/** The height of the app: its host's size container, else the small viewport. */
const VIEWPORT_HEIGHT = "100cqh";

/** The styles of a sidebar and its elements. */
const styles = style.create({
    wrapper: {
        display: "flex",
        minHeight: VIEWPORT_HEIGHT,
        width: "100%",
    },
    sidebar: {
        position: "sticky",
        top: 0,
        display: "flex",
        flexDirection: "column",
        flexShrink: 0,
        height: VIEWPORT_HEIGHT,
        overflow: "hidden",
        backgroundColor: color.sidebar,
        color: color.foreground,
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
        height: `calc(${VIEWPORT_HEIGHT} - 2 * ${space[2]})`,
        margin: space[2],
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[4],
    },
    inner: {
        display: "flex",
        flexDirection: "column",
        width: width.sidebar,
        height: "100%",
    },
    expanded: { width: width.sidebar },
    sized: (pixels: number) => ({ width: `${String(pixels)}px` }),
    resizing: { transitionDuration: "0s" },
    offcanvas: { width: 0, borderWidth: 0 },
    icon: { width: width.sidebarIcon },
    sheet: { width: width.sidebar, padding: 0 },
    rail: {
        position: "absolute",
        top: 0,
        bottom: 0,
        zIndex: 1,
        width: space[4],
        right: { default: 0, [style.when.ancestor('[data-side="right"]')]: "auto" },
        left: { default: "auto", [style.when.ancestor('[data-side="right"]')]: 0 },
        cursor: "ew-resize",
        borderStyle: "solid",
        borderColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.border },
        },
        borderWidth: 0,
        borderRightWidth: { default: stroke.ring, [style.when.ancestor('[data-side="right"]')]: 0 },
        borderLeftWidth: { default: 0, [style.when.ancestor('[data-side="right"]')]: stroke.ring },
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
    input: {
        width: "100%",
        backgroundColor: color.background,
        boxShadow: "none",
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
        gap: stroke.border,
    },
    menuItem: { position: "relative" },
    menuButton: {
        display: "flex",
        alignItems: "center",
        // show the icon alone in a square while the sidebar collapses to its icons
        gap: { default: space[2], [style.when.ancestor('[data-collapsible="icon"]')]: space[6] },
        width: { default: "100%", [style.when.ancestor('[data-collapsible="icon"]')]: space[6] },
        overflow: "hidden",
        paddingInline: space[2],
        borderRadius: radius[3],
        color: {
            default: color.mutedForeground,
            ":hover": { default: null, [media.hover]: color.foreground },
        },
        textAlign: "start",
        whiteSpace: "nowrap",
        cursor: "pointer",
    },
    active: { color: color.foreground },
    menuAction: { position: "absolute", top: space[1], right: space[1] },
    menuBadge: {
        position: "absolute",
        display: "flex",
        alignItems: "center",
        insetBlock: 0,
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
    skeletonWidth: (textWidth: string) => ({
        width: textWidth,
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
    /** Whether the sidebar is open on wide screens, controlled or the provider's own. */
    readonly isOpen: Accessor<boolean>;
    /** Whether the viewport is narrow enough to open the sidebar as a sheet. */
    readonly isMobile: Accessor<boolean>;
    /** Whether the sheet of a narrow viewport is open. */
    readonly isMobileOpen: Accessor<boolean>;
    /** The properties of the provider, read for its controlled state and change handler. */
    readonly #properties: SidebarProviderProperties;
    /** Replace the open state on wide screens. */
    readonly #setOpen: (isOpen: boolean) => void;
    /** Replace the open state of the sheet. */
    readonly #setMobileOpen: (isOpen: boolean) => void;
    /** Replace whether the viewport is narrow. */
    readonly #setMobile: (isMobile: boolean) => void;
    /** The page's environment, whose display the sidebar follows. */
    readonly #environment: Environment<"display">;
    /** The open state the viewer left the sidebar in. */
    readonly #stored: StoredItem;
    /** The width the viewer dragged the sidebar to in pixels, the theme's while never dragged. */
    readonly width: Accessor<number | undefined>;
    /** Whether the sidebar follows a drag of its rail, its width transition off meanwhile. */
    readonly isResizing: Accessor<boolean>;
    /** Replace the dragged width. */
    readonly #setWidth: Setter<number | undefined>;
    /** Replace whether the sidebar follows a drag. */
    readonly #setResizing: Setter<boolean>;
    /** The width the viewer left the sidebar at. */
    readonly #storedWidth: StoredItem;

    /** Create the state of a sidebar. */
    constructor(properties: SidebarProviderProperties) {
        // start from the defaults a server renders too
        const [isOpen, setOpen] = createControllableSignal({
            isControlled: () => properties.open !== undefined,
            value: () => properties.open === true,
            defaultValue: properties.defaultOpen !== false,
        });
        const [isMobileOpen, setMobileOpen] = createSignal(false);
        const [isMobile, setMobile] = createSignal(false);
        const [sidebarWidth, setWidth] = createSignal<number | undefined>(undefined, {
            ownedWrite: true,
        });
        const [isResizing, setResizing] = createSignal(false, { ownedWrite: true });

        // store the state
        this.id = createUniqueId();
        this.isOpen = isOpen;
        this.isMobile = isMobile;
        this.isMobileOpen = isMobileOpen;
        this.#properties = properties;
        this.#setOpen = setOpen;
        this.#setMobileOpen = setMobileOpen;
        this.#setMobile = setMobile;
        const environment = useEnvironment();
        const telemetry = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
        this.#environment = environment;
        this.#stored = new StoredItem(STORAGE_KEY, environment, telemetry);
        this.width = sidebarWidth;
        this.isResizing = isResizing;
        this.#setWidth = setWidth;
        this.#setResizing = setResizing;
        this.#storedWidth = new StoredItem(WIDTH_KEY, environment, telemetry);
    }

    /** Take the state the viewer left the sidebar in, and follow the viewport's width. */
    follow(): () => void {
        // start as the viewer left it, at the width they dragged it to
        const stored = this.#stored.read();
        if (stored !== undefined) {
            this.#setOpen(stored === "true");
        }
        const storedWidth = this.#storedWidth.read();
        if (storedWidth !== undefined) {
            this.#setWidth(this.#clamp(StoredWidth.parse(Number(storedWidth))));
        }

        // follow the viewport's width
        const query = this.#environment.display.media(MOBILE_QUERY);
        const follow = (event: MediaQueryListEvent) => this.#setMobile(event.matches);
        this.#setMobile(query.matches);
        query.addEventListener("change", follow);

        return () => query.removeEventListener("change", follow);
    }

    /** Open or close the sidebar on wide screens and tell the change handler. */
    setOpen(isOpen: boolean): void {
        this.#setOpen(isOpen);
        this.#properties.onOpenChange?.(isOpen);
        this.#stored.write(String(isOpen));
    }

    /** Open or close the sheet of a narrow viewport. */
    setMobileOpen(isOpen: boolean): void {
        this.#setMobileOpen(isOpen);
    }

    /** Follow a drag to a width in pixels, collapsing below half the narrowest. */
    resize(pixels: number): void {
        // collapse a sidebar dragged well past its narrowest
        if (pixels < this.#minimum() / 2) {
            if (this.isOpen()) {
                this.setOpen(false);
            }
        }
        // open a collapsed sidebar dragged back out, at a width within its limits
        else {
            if (!this.isOpen()) {
                this.setOpen(true);
            }
            this.#setWidth(this.#clamp(pixels));
        }
    }

    /** Start or end following a drag, remembering the width it ends at. */
    setResizing(isResizing: boolean): void {
        this.#setResizing(isResizing);
        const dragged = this.width();
        if (!isResizing && dragged !== undefined) {
            this.#storedWidth.write(String(dragged));
        }
    }

    /** Keep a width within the sidebar's limits. */
    #clamp(pixels: number): number {
        return Math.min(Math.max(pixels, this.#minimum()), this.#properties.maxWidth ?? MAX_WIDTH);
    }

    /** Read the narrowest the sidebar gets. */
    #minimum(): number {
        return this.#properties.minWidth ?? MIN_WIDTH;
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
    "class"
> {
    /** Whether the sidebar is open, which makes the state controlled. */
    readonly open?: boolean;
    /** Whether the sidebar starts open when uncontrolled, true by default. */
    readonly defaultOpen?: boolean;
    /** Handle the sidebar opening or closing on wide screens. */
    readonly onOpenChange?: (open: boolean) => void;
    /** The narrowest a dragged sidebar gets in pixels, 208 by default. */
    readonly minWidth?: number;
    /** The widest a dragged sidebar gets in pixels, 400 by default. */
    readonly maxWidth?: number;
    /** The StyleX styles applied after the wrapper's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of a sidebar, the native element's attributes included. */
export interface SidebarProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "id"
> {
    /** The side of the screen, left by default. */
    readonly side?: SidebarSide;
    /** The look, against the edge by default. */
    readonly variant?: SidebarVariant;
    /** How it collapses, off the screen by default. */
    readonly collapsible?: SidebarCollapsible;
    /** The StyleX styles applied after the sidebar's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of an element of a sidebar, the native element's attributes included. */
export type SidebarElementProperties<Attributes> = Omit<Attributes, "class"> &
    ElementPartProperties;

/** The properties of a sidebar menu button, a link with `href` and a button otherwise. */
export type SidebarMenuButtonProperties = SidebarElementProperties<
    JSX.ButtonHTMLAttributes<HTMLButtonElement> & JSX.AnchorHTMLAttributes<HTMLAnchorElement>
> & {
    /** The link target, which renders the button as a link. */
    readonly href?: string;
    /** Whether the button stands for the current page. */
    readonly active?: boolean;
    /** The height of the button, default by default. */
    readonly size?: "default" | "sm" | "lg";
    /** The name a tooltip shows while the sidebar is collapsed to its icons. */
    readonly tooltip?: string;
    /** Render another element with the button's attributes, such as a router's link. */
    readonly render?: Render;
};

/** The properties of a sidebar sub-entry's link, the native anchor's attributes included. */
export type SidebarMenuSubButtonProperties = SidebarElementProperties<
    JSX.AnchorHTMLAttributes<HTMLAnchorElement>
> & {
    /** Whether the link stands for the current page. */
    readonly active?: boolean;
    /** Render another element with the link's attributes, such as a router's link. */
    readonly render?: Render;
};

/** Read the sidebar state of the nearest provider, refusing elements outside one. */
export function useSidebar(): SidebarControl {
    const control = useContext(SidebarContext);
    if (control === null) {
        throw new TypeError("sidebar elements need a sidebar provider around them");
    }

    return control;
}

/** Keep the open state of a page's sidebar. */
export function SidebarProvider(properties: SidebarProviderProperties): JSX.Element {
    // share one state with the sidebar
    const control = new SidebarControl(properties);
    const rest = omit(
        properties,
        "open",
        "defaultOpen",
        "onOpenChange",
        "minWidth",
        "maxWidth",
        "xstyle",
        "style",
    );

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
                {...style.attributes([styles.wrapper, properties.xstyle], properties.style)}
            />
        </SidebarContext>
    );
}

/** Render the sidebar. */
export function Sidebar(properties: SidebarProperties): JSX.Element {
    // read the state and the look
    const control = useSidebar();
    const locale = useLocale();
    const sidebar = merge(DEFAULTS, properties);
    const rest = omit(sidebar, "side", "variant", "collapsible", "xstyle", "style", "children");
    const dragged = () => {
        const pixels = control.width();

        return pixels === undefined ? styles.expanded : styles.sized(pixels);
    };
    const sizing = () =>
        control.isOpen() || sidebar.collapsible === "none"
            ? dragged()
            : styles[sidebar.collapsible === "icon" ? "icon" : "offcanvas"];

    return (
        <Show
            when={!control.isMobile() || sidebar.collapsible === "none"}
            fallback={
                <Sheet
                    open={control.isMobileOpen()}
                    onOpenChange={(isOpen) => control.setMobileOpen(isOpen)}
                >
                    <SheetContent
                        side={sidebar.side}
                        data-slot="sidebar"
                        data-mobile="true"
                        xstyle={styles.sheet}
                    >
                        <SheetTitle xstyle={visuallyHiddenStyle()}>
                            {locale.render(t`Sidebar`)}
                        </SheetTitle>
                        <div {...style.attrs(styles.inner)}>{sidebar.children}</div>
                    </SheetContent>
                </Sheet>
            }
        >
            <div
                id={control.id}
                data-slot="sidebar"
                data-open={control.isOpen() ? "" : undefined}
                data-closed={control.isOpen() ? undefined : ""}
                data-collapsible={control.isOpen() ? undefined : sidebar.collapsible}
                data-variant={sidebar.variant}
                data-side={sidebar.side}
                inert={!control.isOpen() && sidebar.collapsible === "offcanvas"}
                {...rest}
                {...style.attributes(
                    [
                        style.defaultMarker(),
                        text.sm,
                        styles.sidebar,
                        styles[sidebar.side],
                        sidebar.variant === "sidebar" ? null : styles.floating,
                        sizing(),
                        control.isResizing() && styles.resizing,
                        sidebar.xstyle,
                    ],
                    sidebar.style,
                )}
            >
                <div {...style.attrs(styles.inner, dragged())}>{sidebar.children}</div>
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
            size="icon"
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

    // resize the sidebar by the pointer's travel from the width it had, mirrored on the right
    let start = 0;
    let sign = 1;
    let isDragged = false;
    const move = new Move(
        {
            onMoveStart: (event) => {
                // start from the sidebar's width, on the side it sits
                const sidebar = event.target.closest<HTMLElement>("[data-slot=sidebar]");
                if (sidebar === null) {
                    throw new TypeError("sidebar rail is invalid: it renders outside a sidebar");
                }
                start = sidebar.getBoundingClientRect().width;
                sign = sidebar.dataset["side"] === "right" ? -1 : 1;
                isDragged = true;
                control.setResizing(true);
            },
            onMove: (event) => control.resize(start + sign * event.offsetX),
            onMoveEnd: () => control.setResizing(false),
        },
        useEnvironment().clock,
    );

    // toggle on a plain click, leaving the click that ends a drag alone, each press starting undragged
    const press = {
        onPointerDown: () => {
            isDragged = false;
        },
        onClick: () => {
            if (isDragged) {
                isDragged = false;
            } else {
                control.toggle();
            }
        },
    };

    return renderPart(
        "button",
        "sidebar-rail",
        properties,
        styles.rail,
        mergeProperties(
            {
                type: "button" as const,
                tabindex: -1,
                get "aria-label"() {
                    return locale.render(t`Toggle sidebar`);
                },
            },
            mergeProperties(move.attributes(), press),
        ),
    );
}

/** Render the main content beside the sidebar. */
export function SidebarInset(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    return renderPart("main", "sidebar-inset", properties, styles.inset);
}

/** Render the top of the sidebar. */
export function SidebarHeader(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return renderPart("div", "sidebar-header", properties, styles.header);
}

/** Render the bottom of the sidebar. */
export function SidebarFooter(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return renderPart("div", "sidebar-footer", properties, styles.footer);
}

/** Render the scrolling middle of the sidebar with its groups. */
export function SidebarContent(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return renderPart("div", "sidebar-content", properties, styles.content);
}

/** Render a search or filter field of the sidebar, an input sized to it. */
export function SidebarInput(properties: InputProperties): JSX.Element {
    return (
        <Input
            data-slot="sidebar-input"
            {...properties}
            xstyle={[styles.input, properties.xstyle]}
        />
    );
}

/** Render a line between parts of the sidebar. */
export function SidebarSeparator(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return renderPart("div", "sidebar-separator", properties, styles.separator, {
        role: "separator",
    });
}

/** Render a group of the sidebar's menus. */
export function SidebarGroup(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return renderPart("div", "sidebar-group", properties, styles.group, { role: "group" });
}

/** Render the label of a group, faded while the sidebar shows only icons. */
export function SidebarGroupLabel(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useSidebar();

    return renderPart("div", "sidebar-group-label", properties, () => [
        text.xs,
        styles.groupLabel,
        !control.isOpen() && !control.isMobile() && styles.fade,
    ]);
}

/** Render a button in the corner of a group, such as one that adds an entry. */
export function SidebarGroupAction(properties: ButtonProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <Button
            variant="ghost"
            size="icon-sm"
            data-slot="sidebar-group-action"
            {...rest}
            xstyle={[styles.groupAction, properties.xstyle]}
        />
    );
}

/** Render the body of a group. */
export function SidebarGroupContent(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return renderPart("div", "sidebar-group-content", properties, null);
}

/** Render a list of menu entries. */
export function SidebarMenu(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLUListElement>>,
): JSX.Element {
    return renderPart("ul", "sidebar-menu", properties, styles.menu);
}

/** Render one entry of a menu. */
export function SidebarMenuItem(
    properties: SidebarElementProperties<JSX.LiHTMLAttributes<HTMLLIElement>>,
): JSX.Element {
    return renderPart("li", "sidebar-menu-item", properties, styles.menuItem);
}

/** Render an entry's button, or its link with `href`. */
export function SidebarMenuButton(properties: SidebarMenuButtonProperties): JSX.Element {
    const rest = omit(properties, "tooltip");

    return (
        <Show when={properties.tooltip} fallback={<SidebarMenuButtonElement {...rest} />}>
            {(tooltip) => (
                <Tooltip delay={0}>
                    <SidebarMenuButtonElement {...rest} />
                    <TooltipContent side="right">{tooltip()}</TooltipContent>
                </Tooltip>
            )}
        </Show>
    );
}

/** Render the button or link of an entry. */
function SidebarMenuButtonElement(
    properties: Omit<SidebarMenuButtonProperties, "tooltip">,
): JSX.Element {
    // read the sidebar and the tooltip, which only a collapsed sidebar shows
    const control = useSidebar();
    const tooltip = useContext(TooltipContext);
    const isHinting = (): boolean => tooltip !== null && !control.isOpen() && !control.isMobile();
    const rest = omit(properties, "active", "size", "xstyle", "style", "render");
    const hint = (element: HTMLElement) => {
        // anchor the tooltip to the hinted element and show it
        tooltip?.setTrigger(element);
        if (isHinting()) {
            tooltip?.open();
        }
    };
    const hover = new Hover({
        onHoverStart: (event) => hint(event.target),
        onHoverEnd: () => tooltip?.close(),
    });
    const part: PartAttributes = merge(
        hover.attributes(),
        {
            "data-slot": "sidebar-menu-button",
            get "data-active"() {
                return properties.active === true ? "" : undefined;
            },
            get "aria-current"() {
                return properties.active === true ? "page" : undefined;
            },
            get "aria-describedby"() {
                return isHinting() ? tooltip?.id : undefined;
            },
            onFocus: (event: PartEvent<Event>) => hint(event.currentTarget),
            onBlur: () => tooltip?.close(),
        },
        () =>
            style.attributes(
                [
                    state.focus,
                    state.hover,
                    styles.menuButton,
                    sizes[properties.size ?? "default"],
                    properties.active === true && [state.current, styles.active],
                    properties.xstyle,
                ],
                properties.style,
            ),
    );

    // render a link for an address, else a button
    return useRender({
        render: properties.render,
        get defaultTagName() {
            return properties.href === undefined ? "button" : "a";
        },
        part: merge(() => (properties.href === undefined ? { type: "button" } : {}), part),
        properties: rest,
    });
}

/** Render placeholder rows of a menu while its entries load, through the shared skeleton. */
export function SidebarMenuSkeleton(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>> & {
        /** Whether the row shows a square in place of an icon. */
        readonly showIcon?: boolean;
    },
): JSX.Element {
    // vary the text's width between rows
    const rest = omit(properties, "showIcon", "xstyle", "style");
    const textWidth = `${50 + (hashOf(createUniqueId()) % 40)}%`;

    return (
        <div
            data-slot="sidebar-menu-skeleton"
            {...rest}
            {...style.attributes([styles.skeleton, properties.xstyle], properties.style)}
        >
            <Show when={properties.showIcon === true}>
                <Skeleton data-sidebar="menu-skeleton-icon" xstyle={styles.skeletonIcon} />
            </Show>
            <div {...style.attrs(styles.skeletonText, styles.skeletonWidth(textWidth))}>
                <Skeleton data-sidebar="menu-skeleton-text" xstyle={styles.skeletonLine} />
            </div>
        </div>
    );
}

/** Render a button at the end of an entry, such as one that opens the entry's menu. */
export function SidebarMenuAction(properties: ButtonProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <Button
            variant="ghost"
            size="icon-sm"
            data-slot="sidebar-menu-action"
            {...rest}
            xstyle={[styles.menuAction, properties.xstyle]}
        />
    );
}

/** Render a count or status at the end of an entry. */
export function SidebarMenuBadge(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return renderPart("div", "sidebar-menu-badge", properties, [text.xs, styles.menuBadge]);
}

/** Render a nested list of an entry's sub-entries. */
export function SidebarMenuSub(
    properties: SidebarElementProperties<JSX.HTMLAttributes<HTMLUListElement>>,
): JSX.Element {
    return renderPart("ul", "sidebar-menu-sub", properties, styles.menuSub);
}

/** Render one sub-entry. */
export function SidebarMenuSubItem(
    properties: SidebarElementProperties<JSX.LiHTMLAttributes<HTMLLIElement>>,
): JSX.Element {
    return renderPart("li", "sidebar-menu-sub-item", properties, styles.menuItem);
}

/** Render a sub-entry's link, marked as the current page when active. */
export function SidebarMenuSubButton(properties: SidebarMenuSubButtonProperties): JSX.Element {
    const rest = omit(properties, "active", "xstyle", "style", "render");
    const part: PartAttributes = merge(
        {
            get "aria-current"() {
                return properties.active === true ? "page" : undefined;
            },
            "data-slot": "sidebar-menu-sub-button",
            get "data-active"() {
                return properties.active === true ? "" : undefined;
            },
        },
        () =>
            style.attributes(
                [
                    state.focus,
                    state.hover,
                    styles.menuButton,
                    sizes.sm,
                    properties.active === true && [state.current, styles.active],
                    properties.xstyle,
                ],
                properties.style,
            ),
    );

    return useRender({
        render: properties.render,
        defaultTagName: "a",
        part: part,
        properties: rest,
    });
}

/** Hash an identifier to a whole number, alike on the server and in the hydrating page. */
function hashOf(id: string): number {
    // mix each character into the FNV-1a hash
    let hash = FNV_OFFSET;
    for (const character of id) {
        hash = Math.imul(hash ^ character.charCodeAt(0), FNV_PRIME) >>> 0;
    }

    return hash;
}
