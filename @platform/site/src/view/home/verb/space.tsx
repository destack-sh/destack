import { Icon } from "@destack/icon";
import { priceOf } from "../figure/price";
import key from "@destack/icon/phosphor/key";
import { present } from "@destack/schema";
import { createSignal } from "@destack/view";
import * as style from "@destack/style";
import { color, motion, stroke } from "@destack/theme/tokens.stylex";
import { ToggleGroup, ToggleGroupItem } from "@destack/ui/toggle-group";
import { Favicon, Glyph } from "../figure/glyph";
import type { Entry, Item } from "../figure/item";
import { entryOf, type Source } from "./source";
import type { Stagger } from "../figure/stagger";
import { type Change, remixOf, sectionOf } from "./apps";
import { AppRail, appStyles, appText } from "../figure/app";
import { browserStyles } from "../figure/browser";
import { AgentAvatar } from "../figure/agent";
import type { Category, Lineup, Tool } from "../tool/tool";
import { palette } from "../../palette.stylex";
import { media } from "@destack/style/media.stylex";

/** A browser narrower than a phone screen, which then takes a phone browser's chrome. */
const NARROW = "@container (max-width: 479px)";

/** Who looks at your space. */
export type Viewer = "Me" | "Agent" | "Public";

/** Everyone who can look at your space, in the order the switcher offers them. */
export const viewers: readonly Viewer[] = ["Me", "Agent", "Public"];

/** One row of what you keep: what a team rents for it, and the app of your space that does it. */
type Holding = {
    /** What a team rents for the row. */
    category: Category;
    /** The app of your space that does the job. */
    destacked: Item;
};

/** What you keep, numbered as the browser marks them, before your personal app's data. */
const holdings: readonly Holding[] = [
    {
        category: "documents",
        destacked: {
            label: "Docs",
            mark: { app: "pages" },
            name: "Pages",
            price: { kind: "included" },
        },
    },
    {
        category: "tables",
        destacked: {
            label: "Databases",
            mark: { app: "table" },
            name: "Tables",
            price: { kind: "included" },
        },
    },
    {
        category: "tasks",
        destacked: {
            label: "Tasks",
            mark: { app: "tasks" },
            name: "Tasks",
            price: { kind: "included" },
        },
    },
    {
        category: "calendar",
        destacked: {
            label: "Calendar",
            mark: { app: "calendar" },
            name: "Calendar",
            price: { kind: "included" },
        },
    },
    {
        category: "chat",
        destacked: {
            label: "Chat",
            mark: { app: "chat" },
            name: "Chat",
            price: { kind: "included" },
        },
    },
    {
        category: "mail",
        destacked: {
            label: "Mail",
            mark: { app: "mail" },
            name: "Mail",
            price: { kind: "included" },
        },
    },
    {
        category: "canvas",
        destacked: {
            label: "Whiteboard",
            mark: { app: "canvas" },
            name: "Canvas",
            price: { kind: "included" },
        },
    },
    {
        category: "workflows",
        destacked: {
            label: "Automation",
            mark: { app: "sync" },
            name: "Workflows",
            price: { kind: "included" },
        },
    },
    {
        category: "builder",
        destacked: {
            label: "Custom",
            mark: { app: "build" },
            name: "Waitlist",
            price: { kind: "included" },
        },
    },
];

/** The row of the personal app your agent connects, after every app of yours. */
export const DATA_ROW = 10;

/** Return the category a team rents a tool for on a row, or none on the personal app's row. */
export function categoryAt(row: number): Category | undefined {
    const job = present(pages[row - 1], "row page").job;

    return job === "data" ? undefined : job;
}

/** Return what you keep, each row rented from its tool in the lineup, the personal app's last. */
export function holdingsWith(source: Source, lineup: Lineup): readonly Entry[] {
    const rented = holdings.map((holding): Entry => {
        const tool = lineup(holding.category);

        return {
            stacked: {
                label: holding.destacked.label,
                mark: { icon: tool.icon, tint: tool.tint },
                name: tool.name,
                price: priceOf(tool.subscription),
            },
            destacked: holding.destacked,
        };
    });

    return rented.toSpliced(DATA_ROW - 1, 0, entryOf(source));
}

/** The changes your agent makes to your apps: those done, the one asked for, and how to ask. */
type Remixing = {
    remixed: readonly number[];
    asking: number | undefined;
    onRemix: (row: number) => void;
};

/** The job a row does: what a team rents a tool for, or your personal app's data. */
export type Job = Category | "data";

/** One page the browser shows: its row, job, path in your space, viewers, and what it reads. */
export type Page = {
    row: number;
    job: Job;
    path: string;
    viewers: readonly Viewer[];
    links: readonly Link[];
};

/** A value a page shows from another app of your space: that app's job and the value. */
type Link = readonly [job: Job, value: string];

/** The pages the browser turns through, one per ledger row. */
export const pages: readonly Page[] = [
    {
        row: 1,
        job: "documents",
        path: "/launch",
        viewers: ["Me", "Agent"],
        links: [
            ["tasks", "1 of 4 done"],
            ["calendar", "Launch week"],
        ],
    },
    {
        row: 2,
        job: "tables",
        path: "/press",
        viewers: ["Me", "Agent"],
        links: [
            ["calendar", "Call Wed 11:00"],
            ["mail", "Last mail Oct 22"],
        ],
    },
    {
        row: 3,
        job: "tasks",
        path: "/tasks",
        viewers: ["Me"],
        links: [
            ["documents", "Launch post"],
            ["tables", "9 subscriptions"],
            ["calendar", "Demo · Wed 11:00"],
            ["chat", "Post in #launch"],
            ["mail", "400 of 400 sent"],
            ["workflows", "Runs Fri 09:00"],
            ["builder", "1,840 waiting"],
        ],
    },
    {
        row: 4,
        job: "calendar",
        path: "/book",
        viewers: ["Me", "Agent", "Public"],
        links: [
            ["tables", "Demo · Ines"],
            ["tasks", "Go live · DST-17"],
            ["mail", "Invites · 400"],
        ],
    },
    {
        row: 5,
        job: "chat",
        path: "/chat",
        viewers: ["Me", "Agent"],
        links: [
            ["documents", "Launch page"],
            ["tasks", "DST-14"],
            ["calendar", "Wed 11:00"],
        ],
    },
    {
        row: 6,
        job: "mail",
        path: "/mail",
        viewers: ["Me", "Agent"],
        links: [["builder", "1,840 waiting"]],
    },
    {
        row: 7,
        job: "canvas",
        path: "/plan",
        viewers: ["Me", "Agent"],
        links: [["tasks", "6 tasks"]],
    },
    {
        row: 8,
        job: "workflows",
        path: "/workflows",
        viewers: ["Me", "Agent"],
        links: [
            ["tables", "press-followup"],
            ["mail", "invite-400"],
            ["builder", "signup.create"],
        ],
    },
    {
        row: 9,
        job: "builder",
        path: "/waitlist",
        viewers: ["Me", "Agent", "Public"],
        links: [
            ["documents", "Launch post"],
            ["chat", "1 · #launch"],
            ["mail", "Batch 1"],
        ],
    },
    {
        row: 10,
        job: "data",
        path: "/data",
        viewers: ["Me", "Agent"],
        links: [["workflows", "Every 15 minutes"]],
    },
];

/** Return the row of the page that does a job. */
export function rowOf(job: Job): number {
    return present(
        pages.find((page) => page.job === job),
        `page for ${job}`,
    ).row;
}

/** A rented site a figure shows on a row in place of fig. 1's: its tab, address and page. */
export type Vendor = Pick<Tool, "icon" | "tint" | "tab" | "site" | "page"> & { row: number };

/** A tab of the browser: its row, mark, colour and title. */
type Tab = readonly [row: number, icon: string, tint: string, title: string];

/** Return the browser's tabs while rented: the figure's vendor, the personal app, or the tool. */
function tabsOf(source: Source, vendor: Vendor | undefined, lineup: Lineup): readonly Tab[] {
    return pages.map((page): Tab => {
        // show the figure's vendor, the personal app, or the lineup's tool
        if (vendor?.row === page.row) {
            return [page.row, vendor.icon, vendor.tint, vendor.tab];
        } else if (page.job === "data") {
            return [page.row, source.icon, source.tint, source.name];
        }
        const tool = lineup(page.job);

        return [page.row, tool.icon, tool.tint, tool.tab];
    });
}

/** Every app's row, for a browser that shows them all. */
export const everyRow: readonly number[] = pages.map((page) => page.row);

/** The glyphs the rail draws above its apps: search and the inbox. */
const RAIL_TOOLS = ["search", "notify"] as const;

/** The apps of your own space on its rail. */
const sections: readonly (readonly [
    row: number,
    name: string,
    icon: string,
    tint: string,
    seen: readonly Viewer[],
])[] = [
    [1, "Pages", "pages", "#3d6fb0", ["Me", "Agent", "Public"]],
    [2, "Tables", "table", "#4f8a5b", ["Me", "Agent"]],
    [3, "Tasks", "tasks", "#c64a17", ["Me"]],
    [4, "Calendar", "calendar", "#c64a17", ["Me", "Agent", "Public"]],
    [5, "Chat", "chat", "#4f8a5b", ["Me", "Agent"]],
    [6, "Mail", "mail", "#3d6fb0", ["Me", "Agent"]],
    [7, "Canvas", "canvas", "#c64a17", ["Me", "Agent"]],
    [8, "Workflows", "sync", "#3d6fb0", ["Me", "Agent"]],
    [9, "Waitlist", "build", "#4f8a5b", ["Me", "Agent", "Public"]],
    [10, "Connections", "cloud", "#3d6fb0", ["Me", "Agent"]],
];

/** Every glyph the rail draws, loaded before the space opens so the tab icons land on them. */
const railGlyphs: readonly string[] = [...RAIL_TOOLS, ...sections.map(([, , glyph]) => glyph)];

/** Draw your space in the cloud in a browser, one page at a time. */
export function SpaceBrowser(properties: {
    name: string;
    rows: readonly number[];
    change: Change;
    isOpenAt: Stagger;
    page: Page;
    viewer: Viewer;
    onViewer: (viewer: Viewer) => void;
    onChoose: (row: number | undefined) => void;
    source: Source;
    lineup: Lineup;
    vendor: Vendor | undefined;
    remix: Remixing | undefined;
}) {
    // read the change your agent can make to the page shown, and whether it has
    const remix = () => remixOf(properties.page.job, properties.source);
    const isRemixed = () => properties.remix?.remixed.includes(properties.page.row) === true;
    const isAsking = () => properties.remix?.asking === properties.page.row;

    // show the page when the viewer may see it, or say who may
    const isOpen = () => properties.isOpenAt(properties.page.row);
    const isAllowed = () => !isOpen() || properties.page.viewers.includes(properties.viewer);
    const site = () =>
        siteOf(properties.page, properties.source, properties.vendor, properties.lineup);
    const host = () => site().split("/")[0] ?? "";

    // show only the apps the browser can turn to, as tabs and on the rail
    const shown = () =>
        tabsOf(properties.source, properties.vendor, properties.lineup).filter(([row]) =>
            properties.rows.includes(row),
        );

    // open the tab overview from the tab count, and choose a tab from it
    const [isOverview, setIsOverview] = createSignal(false);
    const chooseTab = (row: number) => {
        setIsOverview(false);
        properties.onChoose(row);
    };

    return (
        <div
            data-component="Browser"
            data-morph={properties.name}
            {...style.attrs(browserStyles.browser)}
        >
            {/* draw the rail's glyphs out of sight, so they are loaded when the space opens */}
            <span aria-hidden="true" {...style.attrs(styles.preload)}>
                {railGlyphs.map((glyph) => (
                    <span
                        style={{ "mask-image": `url(/diagram/${glyph}.svg)` }}
                        {...style.attrs(styles.preloadGlyph)}
                    />
                ))}
            </span>
            <div data-component="TabStrip" {...style.attrs(browserStyles.strip)}>
                <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                {properties.isOpenAt(1) ? (
                    <span
                        data-component="Tab"
                        style={{
                            "view-transition-name": `${properties.name}-tab-${String(properties.page.row)}`,
                        }}
                        {...style.attrs(browserStyles.tab, browserStyles.tabOn)}
                    >
                        <Favicon icon="user" tint="#2f7d8c" />
                        <span {...style.attrs(browserStyles.tabTitle)}>Florian</span>
                        <Glyph name="close" size={12} />
                    </span>
                ) : (
                    shown().map(([row, icon, tint, title]) => (
                        <span
                            data-component="Tab"
                            title={title}
                            style={{
                                "view-transition-name": `${properties.name}-tab-${String(row)}`,
                            }}
                            onClick={() => properties.onChoose(row)}
                            {...style.attrs(
                                browserStyles.tab,
                                browserStyles.tabEven,
                                row === properties.page.row && browserStyles.tabOn,
                                properties.rows.length <= 4 && browserStyles.tabSteady,
                            )}
                        >
                            {/* carry the tab's icon to its app on the rail as the space opens */}
                            <span
                                data-morph-icon=""
                                style={{
                                    "view-transition-name": `${properties.name}-icon-${appAt(row)}`,
                                }}
                                {...style.attrs(browserStyles.tabIcon)}
                            >
                                <Favicon icon={icon} tint={tint} />
                            </span>
                            <span
                                {...style.attrs(
                                    browserStyles.tabTitle,
                                    row !== properties.page.row &&
                                        properties.rows.length > 4 &&
                                        browserStyles.tabTitleHidden,
                                )}
                            >
                                {title}
                            </span>
                        </span>
                    ))
                )}
                <span {...style.attrs(browserStyles.newTab)}>
                    <Glyph name="plus" />
                </span>
            </div>
            <div data-component="Toolbar" {...style.attrs(browserStyles.toolbar)}>
                <span {...style.attrs(browserStyles.tools, browserStyles.desktopOnly)}>
                    <Glyph name="back" />
                    <Glyph name="forward" />
                    <Glyph name="reload" />
                </span>
                <span data-component="AddressBar" {...style.attrs(browserStyles.omnibox)}>
                    <Glyph name="lock" size={12} />
                    {isOpen() ? (
                        <span {...style.attrs(browserStyles.url)}>
                            <span {...style.attrs(browserStyles.host)}>destack.app</span>
                            <span {...style.attrs(browserStyles.path)}>
                                /@florian{properties.page.path === "/" ? "" : properties.page.path}
                            </span>
                        </span>
                    ) : (
                        <span {...style.attrs(browserStyles.url)}>
                            <span {...style.attrs(browserStyles.host)}>{host()}</span>
                            <span {...style.attrs(browserStyles.path)}>
                                {site().slice(host().length)}
                            </span>
                        </span>
                    )}
                    <Glyph name="star" />
                </span>
                {/* count the open tabs as a phone's browser does */}
                <button
                    type="button"
                    data-component="TabCount"
                    aria-label="Show tabs"
                    aria-expanded={isOverview() ? "true" : "false"}
                    onClick={() => setIsOverview((is) => !is)}
                    {...style.attrs(browserStyles.tabCount)}
                >
                    {isOpen() ? 1 : shown().length}
                </button>
                <span {...style.attrs(browserStyles.tools)}>
                    <span {...style.attrs(browserStyles.desktopOnly)}>
                        <Glyph name="puzzle" />
                    </span>
                    <span {...style.attrs(browserStyles.profile, browserStyles.desktopOnly)}>
                        F
                    </span>
                    <Glyph name="menu" />
                </span>
            </div>
            {/* show the open tabs as cards over the page while the overview is open */}
            <div
                data-component="TabOverview"
                {...style.attrs(styles.overview, !isOverview() && styles.overviewClosed)}
            >
                <span {...style.attrs(styles.overviewHead)}>
                    <b>{isOpen() ? "1 tab" : `${shown().length} tabs`}</b>
                    <button
                        type="button"
                        onClick={() => setIsOverview(false)}
                        {...style.attrs(styles.overviewDone)}
                    >
                        Done
                    </button>
                </span>
                <div {...style.attrs(styles.cards)}>
                    {isOpen() ? (
                        <button
                            type="button"
                            onClick={() => chooseTab(properties.page.row)}
                            {...style.attrs(styles.card, styles.cardOn)}
                        >
                            <span {...style.attrs(styles.cardTitle)}>
                                <Favicon icon="user" tint="#2f7d8c" />
                                Florian
                            </span>
                            <span {...style.attrs(styles.cardHost)}>destack.app</span>
                        </button>
                    ) : (
                        shown().map(([row, icon, tint, title]) => (
                            <button
                                type="button"
                                onClick={() => chooseTab(row)}
                                {...style.attrs(
                                    styles.card,
                                    row === properties.page.row && styles.cardOn,
                                )}
                            >
                                <span {...style.attrs(styles.cardTitle)}>
                                    <Favicon icon={icon} tint={tint} />
                                    {title}
                                </span>
                                <span {...style.attrs(styles.cardHost)}>
                                    {hostOf(
                                        row,
                                        properties.source,
                                        properties.vendor,
                                        properties.lineup,
                                    )}
                                </span>
                            </button>
                        ))
                    )}
                </div>
            </div>
            <div
                data-component={isOpen() ? "SitePage" : "RentedPage"}
                {...style.attrs(browserStyles.viewport)}
            >
                {isOpen() ? (
                    <div {...style.attrs(appStyles.app, styles.owned)}>
                        <AppRail
                            open={appAt(properties.page.row)}
                            apps={sections
                                .filter(
                                    ([row, , , , seen]) =>
                                        properties.rows.includes(row) &&
                                        seen.includes(properties.viewer),
                                )
                                .map(([, name, icon]) => [icon, name] as const)}
                            morph={properties.name}
                            onOpen={(icon) => properties.onChoose(rowOfApp(icon))}
                        />
                        <main
                            style={{ "view-transition-name": `${properties.name}-page` }}
                            {...style.attrs(appStyles.main)}
                        >
                            {isAllowed() ? (
                                sectionOf(
                                    properties.page.job,
                                    properties.change,
                                    properties.source,
                                    isRemixed(),
                                )
                            ) : (
                                <p data-component="AccessNote" {...style.attrs(styles.private)}>
                                    <Glyph name="lock" size={20} />
                                    {properties.page.viewers.length === 1
                                        ? "Only you can see this page"
                                        : "Only you and your agent can see this page"}
                                </p>
                            )}
                        </main>
                        {/* your agent's change to the page, asked and then done */}
                        {remix() !== undefined && (isAsking() || isRemixed()) ? (
                            <p data-component="RemixNote" {...style.attrs(styles.remixNote)}>
                                <AgentAvatar size="small" />
                                {isRemixed() ? remix()?.done : remix()?.ask}
                            </p>
                        ) : undefined}
                        <SpaceStatus
                            page={properties.page}
                            viewer={properties.viewer}
                            onViewer={properties.onViewer}
                            remix={remix() === undefined ? undefined : properties.remix}
                        />
                    </div>
                ) : (
                    <div
                        style={{ "view-transition-name": `${properties.name}-page` }}
                        {...style.attrs(styles.window)}
                    >
                        {properties.vendor?.row === properties.page.row
                            ? properties.vendor.page()
                            : properties.page.job === "data"
                              ? properties.source.page()
                              : properties.lineup(properties.page.job).page()}
                    </div>
                )}
            </div>
        </div>
    );
}

/** Return the ledger row of the app of your space with a glyph. */
function rowOfApp(icon: string): number {
    return present(
        sections.find((section) => section[2] === icon),
        "app on the rail",
    )[0];
}

/** Return the address host of the rented site on a ledger row. */
function hostOf(row: number, source: Source, vendor: Vendor | undefined, lineup: Lineup): string {
    const page = present(
        pages.find((candidate) => candidate.row === row),
        "page of a tab",
    );

    return present(siteOf(page, source, vendor, lineup).split("/")[0], "host of a site");
}

/** Return the address of the rented site a page shows: its own, or the personal app's. */
function siteOf(page: Page, source: Source, vendor: Vendor | undefined, lineup: Lineup): string {
    // read the figure's own vendor, the personal app's, or the lineup's tool
    if (vendor?.row === page.row) {
        return vendor.site;
    } else if (page.job === "data") {
        return source.site;
    }

    return lineup(page.job).site;
}

/** Return the glyph of the app of your space on a ledger row. */
function appAt(row: number): string {
    return present(
        sections.find(([candidate]) => candidate === row),
        "rail app of a row",
    )[2];
}

/** Draw the status bar every app of your space shares. */
function SpaceStatus(properties: {
    page: Page;
    viewer: Viewer;
    onViewer: (viewer: Viewer) => void;
    remix: Remixing | undefined;
}) {
    return (
        <div data-component="StatusBar" {...style.attrs(appStyles.statusBar)}>
            <span data-component="AudienceChip" {...style.attrs(appStyles.status)}>
                <Icon icon={key} />
                {audienceOf(properties.page)}
            </span>
            <span {...style.attrs(appStyles.statusEnd)}>
                {/* ask your agent to change the app on the page */}
                {properties.remix === undefined ? undefined : (
                    <button
                        type="button"
                        data-component="Remix"
                        onClick={() => properties.remix?.onRemix(properties.page.row)}
                        {...style.attrs(appText.small, styles.remix)}
                    >
                        <Glyph name="remix" size={12} />
                        Remix
                    </button>
                )}
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

/** The space styles. */
const styles = style.create({
    preload: {
        height: "1px",
        opacity: 0,
        overflow: "hidden",
        pointerEvents: "none",
        position: "absolute",
        width: "1px",
    },
    preloadGlyph: {
        display: "block",
        height: "1px",
        width: "1px",
    },
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
    remix: {
        alignItems: "center",
        backgroundColor: `color-mix(in srgb, ${palette.signal} 14%, transparent)`,
        borderColor: `color-mix(in srgb, ${palette.signal} 45%, transparent)`,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        color: color.foreground,
        cursor: "pointer",
        display: "inline-flex",
        gap: "0.3125rem",
        height: "1.5rem",
        paddingInline: "0.5rem",
    },
    remixNote: {
        alignItems: "center",
        backgroundColor: color.card,
        borderColor: color.border,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        bottom: "3rem",
        boxShadow: "0 4px 16px color-mix(in srgb, black 12%, transparent)",
        display: "flex",
        fontSize: "0.8125rem",
        gap: "0.5rem",
        insetInlineEnd: "1rem",
        margin: 0,
        paddingBlock: "0.375rem",
        paddingInlineEnd: "0.875rem",
        paddingInlineStart: "0.5rem",
        position: "absolute",
        zIndex: 2,
    },
    overview: {
        alignContent: "start",
        backgroundColor: color.muted,
        display: "grid",
        gap: "0.75rem",
        gridColumn: "1 / -1",
        gridRow: { default: "3", [NARROW]: "2" },
        minHeight: 0,
        overflowY: "auto",
        padding: "0.75rem",
        transitionDuration: motion.durationShort,
        transitionProperty: "opacity, visibility",
        transitionTimingFunction: motion.easingStandard,
        zIndex: 2,
    },
    overviewClosed: {
        opacity: 0,
        pointerEvents: "none",
        visibility: "hidden",
    },
    overviewHead: {
        alignItems: "center",
        display: "flex",
        fontSize: "0.875rem",
        justifyContent: "space-between",
    },
    overviewDone: {
        backgroundColor: "transparent",
        textDecorationLine: {
            default: null,
            ":hover": { default: null, [media.hover]: "underline" },
        },
        borderWidth: 0,
        color: color.primary,
        cursor: "pointer",
        fontSize: "0.875rem",
        fontWeight: 600,
        padding: 0,
    },
    cards: {
        display: "grid",
        gap: "0.5rem",
        gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
    },
    card: {
        alignContent: "space-between",
        backgroundColor: {
            default: color.card,
            ":hover": {
                default: null,
                [media.hover]: `color-mix(in srgb, ${color.foreground} 4%, ${color.card})`,
            },
        },
        borderColor: color.border,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        color: color.cardForeground,
        cursor: "pointer",
        display: "grid",
        fontSize: "0.75rem",
        gap: "0.5rem",
        height: "4.5rem",
        paddingBlock: "0.625rem",
        paddingInline: "0.625rem",
        textAlign: "start",
    },
    cardOn: {
        borderColor: color.primary,
        boxShadow: `0 0 0 1px ${color.primary}`,
    },
    cardTitle: {
        alignItems: "center",
        display: "flex",
        fontWeight: 600,
        gap: "0.375rem",
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    cardHost: {
        color: color.mutedForeground,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
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
