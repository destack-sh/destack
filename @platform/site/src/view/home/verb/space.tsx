import { Icon } from "@destack/icon";
import key from "@destack/icon/phosphor/key";
import { present } from "@destack/schema";
import { createEffect, createSignal, onSettled } from "@destack/view";
import * as style from "@destack/style";
import { color, shadow, stroke } from "@destack/theme/tokens.stylex";
import { ToggleGroup, ToggleGroupItem } from "@destack/ui/toggle-group";
import { Fade } from "../figure/fade";
import { Favicon, Glyph } from "../figure/glyph";
import type { Entry } from "../figure/ledger";
import type { Stagger } from "../figure/stagger";
import { sectionOf } from "./apps";
import { AppRail, appStyles, appText } from "../figure/app";
import { rentedPageOf } from "./vendor";
import { palette } from "../../palette.stylex";
import { media } from "@destack/style/media.stylex";
import { makeResizeObserver } from "@destack/view/primitives/resize-observer";

/** Who looks at your space. */
export type Viewer = "Me" | "Agent" | "Public";

/** Everyone who can look at your space, in the order the switcher offers them. */
export const viewers: readonly Viewer[] = ["Me", "Agent", "Public"];

/** What you keep, numbered as the browser marks them: rented as a site each, or held in your own space. */
export const holdings: readonly Entry[] = [
    {
        stacked: {
            label: "Launch plan",
            mark: { icon: "notion", tint: "#191919" },
            name: "Notion",
            chips: ["public link"],
            note: "anyone with the link can read it",
        },
        destacked: {
            label: "Launch plan",
            mark: { icon: "pages", tint: "#3d6fb0" },
            name: "Pages",
            chips: ["agent"],
            note: "only the people you pick can read it",
        },
    },
    {
        stacked: {
            label: "Tasks",
            mark: { icon: "linear", tint: "#5e6ad2" },
            name: "Linear",
            chips: ["per seat"],
            note: "a paid seat for every viewer",
        },
        destacked: {
            label: "Tasks",
            mark: { icon: "tasks", tint: "#c64a17" },
            name: "Tasks",
            chips: ["Me"],
            note: "a free seat for every viewer",
        },
    },
    {
        stacked: {
            label: "Chat",
            mark: { icon: "slack", tint: "#4a154b" },
            name: "Slack",
            chips: ["guest invite"],
            note: "a guest invite for every channel",
        },
        destacked: {
            label: "Chat",
            mark: { icon: "chat", tint: "#4f8a5b" },
            name: "Chat",
            chips: ["agent"],
            note: "one sign-in for every channel",
        },
    },
    {
        stacked: {
            label: "Code",
            mark: { icon: "github", tint: "#181717" },
            name: "GitHub",
            chips: ["separate repo"],
            note: "code kept apart from the apps",
        },
        destacked: {
            label: "Code",
            mark: { icon: "source", tint: "#c64a17" },
            name: "Forge",
            chips: ["fork"],
            note: "code kept beside the apps",
        },
    },
    {
        stacked: {
            label: "Website",
            mark: { icon: "framer", tint: "#0a0a0a" },
            name: "Framer",
            chips: ["their domain"],
            note: "edited only in their builder",
        },
        destacked: {
            label: "Website",
            mark: { icon: "pages", tint: "#3d6fb0" },
            name: "Pages",
            chips: ["public"],
            note: "edited like any other page",
        },
    },
    {
        stacked: {
            label: "Booking",
            mark: { icon: "calendly", tint: "#006bff" },
            name: "Calendly",
            chips: ["synced calendar"],
            note: "free times copied, never live",
        },
        destacked: {
            label: "Booking",
            mark: { icon: "calendar", tint: "#c64a17" },
            name: "Calendar",
            chips: ["public"],
            note: "free times read live, never copied",
        },
    },
    {
        stacked: {
            label: "Waitlist",
            mark: { icon: "replit", tint: "#f26207" },
            name: "Replit",
            chips: ["own database"],
            note: "signups in a database of its own",
        },
        destacked: {
            label: "Waitlist",
            mark: { icon: "table", tint: "#4f8a5b" },
            name: "Waitlist",
            chips: ["built"],
            note: "signups in the tables you already have",
        },
    },
];

/** One page the browser shows: its ledger row, the rail row of the app that opens it, its path in your space, who sees it once owned, and the rented site's address. */
export type Page = {
    row: number;
    app: number;
    path: string;
    viewers: readonly Viewer[];
    site: string;
};

/** The pages the browser turns through, one per ledger row. */
export const pages: readonly Page[] = [
    {
        row: 1,
        app: 1,
        path: "/launch",
        viewers: ["Me", "Agent"],
        site: "florian.notion.site/Launch-plan",
    },
    { row: 2, app: 2, path: "/tasks", viewers: ["Me"], site: "linear.app/launch/team/LCH/all" },
    {
        row: 3,
        app: 3,
        path: "/chat",
        viewers: ["Me", "Agent"],
        site: "launchkit.slack.com/archives/launch",
    },
    {
        row: 4,
        app: 4,
        path: "/code",
        viewers: ["Me", "Agent"],
        site: "github.com/florian/launchkit",
    },
    {
        row: 5,
        app: 1,
        path: "/",
        viewers: ["Me", "Agent", "Public"],
        site: "florian.framer.website",
    },
    {
        row: 6,
        app: 6,
        path: "/book",
        viewers: ["Me", "Agent", "Public"],
        site: "calendly.com/florian/30min",
    },
    {
        row: 7,
        app: 7,
        path: "/waitlist",
        viewers: ["Me", "Agent", "Public"],
        site: "florian-waitlist.replit.app",
    },
];

/** The sites you rent, as the browser's tabs while rented: the ledger row, the icon and its tint, and the tab's title. */
const tabs: readonly (readonly [row: number, icon: string, tint: string, title: string])[] = [
    [1, "notion", "#191919", "Launch plan"],
    [2, "linear", "#5e6ad2", "Launch › All issues"],
    [3, "slack", "#4a154b", "launch (Channel) - LaunchKit - Slack"],
    [4, "github", "#181717", "florian/launchkit"],
    [5, "framer", "#0a0a0a", "Florian · Portfolio"],
    [6, "calendly", "#006bff", "30 Minute Meeting | Florian | Calendly"],
    [7, "replit", "#f26207", "Join the waitlist"],
];

/** The apps of your own space on its rail: the ledger row, the name, the icon and its tint, and who sees it. */
const sections: readonly (readonly [
    row: number,
    name: string,
    icon: string,
    tint: string,
    seen: readonly Viewer[],
])[] = [
    [1, "Pages", "pages", "#3d6fb0", ["Me", "Agent", "Public"]],
    [2, "Tasks", "tasks", "#c64a17", ["Me"]],
    [3, "Chat", "chat", "#4f8a5b", ["Me", "Agent"]],
    [4, "Forge", "source", "#c64a17", ["Me", "Agent"]],
    [6, "Calendar", "calendar", "#c64a17", ["Me", "Agent", "Public"]],
    [7, "Tables", "table", "#4f8a5b", ["Me", "Agent", "Public"]],
];

/**
 * Draw your corner of the cloud in a browser, one page at a time.
 *
 * While rented, every page is a different site in its own tab, with its own look, login and sharing.
 * Once owned, every page is an app of one space at your own handle, shown to whoever looks as far as access allows.
 * Every page lays out at the width of a real window and shows at four fifths of its size.
 */
export function SpaceBrowser(properties: {
    isOpenAt: Stagger;
    page: Page;
    viewer: Viewer;
    onViewer: (viewer: Viewer) => void;
}) {
    // show the page when the viewer may see it, or say who may
    const isOpen = () => properties.isOpenAt(properties.page.row);
    const isAllowed = () => !isOpen() || properties.page.viewers.includes(properties.viewer);
    const host = () => properties.page.site.split("/")[0] ?? "";

    return (
        <div data-component="Browser" {...style.attrs(styles.browser)}>
            <div data-component="TabStrip" {...style.attrs(styles.strip)}>
                <span aria-hidden="true" {...style.attrs(styles.lights)} />
                {properties.isOpenAt(1) ? (
                    <span
                        data-component="Tab"
                        data-service="1"
                        {...style.attrs(styles.tab, styles.tabOn)}
                    >
                        <Favicon icon="user" tint="#2f7d8c" />
                        <span {...style.attrs(styles.tabTitle)}>Florian</span>
                        <Glyph name="close" size={12} />
                    </span>
                ) : (
                    tabs.map(([row, icon, tint, title]) => (
                        <span
                            data-component="Tab"
                            data-service={String(row)}
                            title={title}
                            {...style.attrs(
                                styles.tab,
                                styles.tabEven,
                                row === properties.page.row && styles.tabOn,
                            )}
                        >
                            <Favicon icon={icon} tint={tint} />
                            <span
                                {...style.attrs(
                                    styles.tabTitle,
                                    row !== properties.page.row && styles.tabTitleHidden,
                                )}
                            >
                                {title}
                            </span>
                        </span>
                    ))
                )}
                <span {...style.attrs(styles.newTab)}>
                    <Glyph name="plus" />
                </span>
            </div>
            <div data-component="Toolbar" {...style.attrs(styles.toolbar)}>
                <span {...style.attrs(styles.tools)}>
                    <Glyph name="back" />
                    <Glyph name="forward" />
                    <Glyph name="reload" />
                </span>
                <span data-component="AddressBar" {...style.attrs(styles.omnibox)}>
                    <Glyph name="lock" size={12} />
                    {isOpen() ? (
                        <span {...style.attrs(styles.url)}>
                            <span {...style.attrs(styles.host)}>destack.app</span>
                            /@florian{properties.page.path === "/" ? "" : properties.page.path}
                        </span>
                    ) : (
                        <span {...style.attrs(styles.url)}>
                            <span {...style.attrs(styles.host)}>{host()}</span>
                            {properties.page.site.slice(host().length)}
                        </span>
                    )}
                    <Glyph name="star" />
                </span>
                <span {...style.attrs(styles.tools)}>
                    <Glyph name="puzzle" />
                    <span {...style.attrs(styles.profile)}>F</span>
                    <Glyph name="menu" />
                </span>
            </div>
            <Fade
                isOpen={isOpen()}
                xstyle={styles.viewport}
                isLarge
                component={isOpen() ? "SitePage" : "RentedPage"}
            >
                {isOpen() ? (
                    <div
                        data-service={String(properties.page.row)}
                        {...style.attrs(appStyles.app, styles.owned)}
                    >
                        <AppRail
                            open={openApp(properties.page)}
                            apps={sections
                                .filter(([, , , , seen]) => seen.includes(properties.viewer))
                                .map(([, name, icon]) => [icon, name] as const)}
                        />
                        <main {...style.attrs(appStyles.main)}>
                            {isAllowed() ? (
                                sectionOf(properties.page.row)
                            ) : (
                                <p data-component="AccessNote" {...style.attrs(styles.private)}>
                                    <Glyph name="lock" size={20} />
                                    {properties.page.viewers.length === 1
                                        ? "Only you can see this page"
                                        : "Only you and your agent can see this page"}
                                </p>
                            )}
                        </main>
                        <SpaceStatus
                            page={properties.page}
                            viewer={properties.viewer}
                            onViewer={properties.onViewer}
                        />
                    </div>
                ) : (
                    <div data-service={String(properties.page.row)} {...style.attrs(styles.window)}>
                        {rentedPageOf(properties.page.row)}
                    </div>
                )}
            </Fade>
        </div>
    );
}

/** Return the glyph of the app a page opens in your space. */
function openApp(page: Page): string {
    return sections.find(([row]) => row === page.app)?.[2] ?? "pages";
}

/** Draw the status bar every app of your space shares: where the page lives, who can see it, and the preview of who is looking. */
function SpaceStatus(properties: {
    page: Page;
    viewer: Viewer;
    onViewer: (viewer: Viewer) => void;
}) {
    return (
        <div data-component="StatusBar" {...style.attrs(appStyles.statusBar)}>
            <span {...style.attrs(appStyles.status)}>
                <span {...style.attrs(appStyles.dot)} />
                Yours · @florian{properties.page.path === "/" ? "" : properties.page.path}
            </span>
            <span data-component="AudienceChip" {...style.attrs(appStyles.status)}>
                <Icon icon={key} />
                {audienceOf(properties.page)}
            </span>
            <span {...style.attrs(appStyles.statusEnd)}>
                <span>View as</span>
                <ToggleGroup
                    data-component="ViewAs"
                    aria-label="View as"
                    variant="outline"
                    size="sm"
                    value={properties.viewer}
                    onValueChange={(value) => {
                        const chosen = viewers.find((viewer) => viewer === value);
                        if (chosen !== undefined) {
                            properties.onViewer(chosen);
                        }
                    }}
                    xstyle={styles.viewAs}
                >
                    {viewers.map((viewer) => (
                        <ToggleGroupItem value={viewer} xstyle={[appText.small, styles.viewer]}>
                            {viewer}
                        </ToggleGroupItem>
                    ))}
                </ToggleGroup>
            </span>
        </div>
    );
}

/** Name who can see a page: you alone, you and your agent, or everyone. */
function audienceOf(page: Page): string {
    if (page.viewers.includes("Public")) {
        return "Public";
    } else if (page.viewers.includes("Agent")) {
        return "You and your agent";
    }

    return "Only you";
}

/** Mark where a row's faces sit, which the travelling presence moves to. */
export function PresenceSlot(properties: { row: number }) {
    return <span data-presence-row={String(properties.row)} {...style.attrs(styles.slot)} />;
}

/** Draw the people in the page the browser shows, travelling to its row: you, your agent where it may look, and the public. */
export function Presence(properties: { row: number; isShown: boolean }) {
    // hold the presence and where its row's slot sits within the ledger
    let element: HTMLSpanElement | undefined;
    const [place, setPlace] = createSignal<{ x: number; y: number }>();
    const page = () => present(pages[properties.row - 1], "row page");

    // move to the slot of the row on show, on each turn of the page and as the ledger resizes
    const measure = (row: number) => {
        // find the row's slot, waiting until the presence and its ledger are laid out
        const host = element?.offsetParent;
        const slot = host?.querySelector(`[data-presence-row="${String(row)}"]`);
        if (!element || !host || !slot) {
            return;
        }

        // align the faces' end with the slot's end, centred on its line
        const bounds = host.getBoundingClientRect();
        const target = slot.getBoundingClientRect();
        setPlace({
            x: target.right - bounds.left - element.offsetWidth,
            y: target.top - bounds.top + (target.height - element.offsetHeight) / 2,
        });
    };
    createEffect(() => properties.row, measure);
    const resizes = makeResizeObserver(() => measure(properties.row));
    onSettled(() => {
        // measure again as the ledger resizes
        const host = element?.offsetParent;
        if (host) {
            resizes.observe(host);
        }
        measure(properties.row);
    });

    return (
        <span
            ref={element}
            data-component="Presence"
            aria-hidden="true"
            style={{
                translate:
                    place() === undefined ? undefined : `${place()?.x ?? 0}px ${place()?.y ?? 0}px`,
                opacity: properties.isShown && place() !== undefined ? "1" : "0",
            }}
            {...style.attrs(styles.presence)}
        >
            {page().viewers.map((viewer) =>
                viewer === "Public" ? (
                    <span title="Public" {...style.attrs(styles.face, styles.facePublic)}>
                        <Glyph name="globe" size={12} />
                    </span>
                ) : (
                    <span
                        title={viewer === "Me" ? "Florian" : "Agent"}
                        style={{
                            "background-color": viewer === "Me" ? palette.teal : palette.violet,
                        }}
                        {...style.attrs(styles.face)}
                    >
                        <span
                            style={{
                                "mask-image": `url(/diagram/${viewer === "Me" ? "user" : "agent"}.svg)`,
                            }}
                            {...style.attrs(styles.faceGlyph)}
                        />
                    </span>
                ),
            )}
        </span>
    );
}

/** The space styles: the browser in the page's units, and its pages in the pixels of a real window. */
const styles = style.create({
    owned: {
        height: "100%",
        inset: 0,
        position: "absolute",
    },
    viewAs: {
        gap: "0.25rem",
    },
    viewer: {
        height: "1.5rem",
        paddingInline: "0.5rem",
    },
    face: {
        alignItems: "center",
        borderColor: color.background,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "2px",
        boxSizing: "content-box",
        display: "inline-flex",
        height: "1.25rem",
        justifyContent: "center",
        marginLeft: "-0.375rem",
        width: "1.25rem",
    },
    faceGlyph: {
        backgroundColor: palette.cream,
        height: "0.75rem",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "0.75rem",
    },
    slot: {
        display: "inline-block",
        height: "1.5rem",
        width: "3.75rem",
    },
    presence: {
        display: "flex",
        insetBlockStart: 0,
        insetInlineStart: 0,
        pointerEvents: "none",
        position: "absolute",
        transition: {
            default: `translate 520ms cubic-bezier(0.3, 0.8, 0.3, 1), opacity 300ms ease`,
            [media.motionReduce]: "none",
        },
        zIndex: 1,
    },
    browser: {
        backgroundColor: color.card,
        borderColor: color.border,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        boxShadow: shadow.raised,
        color: color.cardForeground,
        display: "grid",
        fontSize: "0.8125rem",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        height: "100%",
        minHeight: 0,
        overflow: "hidden",
    },
    strip: {
        alignItems: "stretch",
        backgroundColor: color.muted,
        display: "flex",
        gap: "0.125rem",
        height: "2.5rem",
        minWidth: 0,
        paddingInlineStart: "0.875rem",
        paddingInlineEnd: "0.625rem",
        paddingTop: "0.375rem",
    },
    lights: {
        alignSelf: "stretch",
        backgroundImage: `radial-gradient(circle at 6px 50%, ${palette.windowClose} 5.5px, transparent 6px), radial-gradient(circle at 26px 50%, ${palette.windowMinimize} 5.5px, transparent 6px), radial-gradient(circle at 46px 50%, ${palette.windowZoom} 5.5px, transparent 6px)`,
        backgroundRepeat: "no-repeat",
        flexShrink: 0,
        marginRight: "0.875rem",
        width: "52px",
    },
    tab: {
        alignItems: "center",
        borderStartStartRadius: "8px",
        borderStartEndRadius: "8px",
        borderEndEndRadius: "0",
        borderEndStartRadius: "0",
        color: color.mutedForeground,
        display: "flex",
        flexShrink: 0,
        fontSize: "0.72rem",
        gap: "0.5rem",
        justifyContent: "center",
        minWidth: 0,
        paddingInline: "0.75rem",
        position: "relative",
    },
    tabEven: {
        flexBasis: 0,
        flexGrow: 1,
        flexShrink: 1,
        justifyContent: "flex-start",
        maxWidth: "12rem",
        transition: {
            default:
                "flex-grow 180ms cubic-bezier(0.23, 1, 0.32, 1), background-color 160ms ease, color 160ms ease",
            [media.motionReduce]: "none",
        },
    },
    tabOn: {
        flexGrow: 5,
        backgroundColor: color.card,
        color: color.foreground,
        "::before": {
            backgroundImage: `radial-gradient(circle at 0 0, transparent 8px, ${color.card} 8.5px)`,
            bottom: 0,
            content: "''",
            height: "8px",
            left: "-8px",
            position: "absolute",
            width: "8px",
        },
        "::after": {
            backgroundImage: `radial-gradient(circle at 100% 0, transparent 8px, ${color.card} 8.5px)`,
            bottom: 0,
            content: "''",
            height: "8px",
            position: "absolute",
            right: "-8px",
            width: "8px",
        },
    },
    tabTitleHidden: {
        opacity: 0,
    },
    tabTitle: {
        flexGrow: 1,
        transition: "opacity 160ms ease",
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    newTab: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        flexShrink: 0,
        paddingInline: "0.5rem",
    },
    toolbar: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "flex",
        gap: "0.875rem",
        height: "2.75rem",
        minWidth: 0,
        paddingInline: "0.875rem",
    },
    tools: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        flexShrink: 0,
        gap: "0.875rem",
    },
    profile: {
        alignItems: "center",
        backgroundColor: palette.teal,
        borderRadius: "50%",
        color: "white",
        display: "inline-flex",
        fontSize: "0.6rem",
        fontWeight: 700,
        height: "1.25rem",
        justifyContent: "center",
        width: "1.25rem",
    },
    omnibox: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: "999px",
        color: color.mutedForeground,
        display: "flex",
        flexGrow: 1,
        gap: "0.5rem",
        height: "1.875rem",
        minWidth: 0,
        paddingInline: "0.875rem",
    },
    url: {
        alignItems: "baseline",
        display: "flex",
        flexGrow: 1,
        minWidth: 0,
        overflow: "hidden",
        whiteSpace: "nowrap",
    },
    host: {
        color: color.foreground,
    },
    viewport: {
        maskImage: "linear-gradient(to bottom, black calc(100% - 1rem), transparent)",
        minHeight: 0,
        overflow: "clip",
        position: "relative",
    },
    window: {
        height: "125%",
        left: 0,
        overflow: "clip",
        position: "absolute",
        top: 0,
        transform: "scale(0.8)",
        transformOrigin: "0 0",
        width: "125%",
    },
    stack: {
        display: "flex",
        paddingLeft: "4px",
    },
    facePublic: {
        backgroundColor: color.muted,
        color: color.mutedForeground,
    },
    private: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        fontSize: "18px",
        gap: "10px",
        justifyContent: "center",
        margin: 0,
        minHeight: "420px",
    },
});
