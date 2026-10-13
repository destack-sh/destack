import { Icon } from "@destack/icon";
import gitCommit from "@destack/icon/phosphor/git-commit";
import * as style from "@destack/style";
import { color, font, stroke } from "@destack/theme/tokens.stylex";
import { createEffect, createSignal, onSettled } from "@destack/view";
import { makeResizeObserver } from "@destack/view/primitives/resize-observer";

import { palette } from "../../palette.stylex";
import { AppRail, appStyles } from "../figure/app";
import { browserStyles } from "../figure/browser";
import { Fade } from "../figure/fade";
import { Favicon, Glyph } from "../figure/glyph";
import { accentOf } from "../figure/item";
import type { Stagger } from "../figure/stagger";
import { ServiceGlyph } from "../figure/tile";
import { sectionOf } from "../verb/apps";
import { services } from "./service";

/** A browser too narrow for the tag column, where each tag pins to its part instead. */
const PINNED = "@container (max-width: 559px)";

/** How each rented service is wired into the app's own code, in ledger order. */
const wiring: readonly string[] = [
    "SDK",
    "CLI · keys",
    "SDK · URL",
    "OAuth",
    "API key",
    "API key",
    "worker",
    "snippet",
    "deploy hook",
    "webhook",
];

/** Your apps, one tab each while rented: their page, glyph, tab title, own subdomain and path. */
const apps: readonly (readonly [
    row: number,
    icon: string,
    title: string,
    slug: string,
    path: string,
])[] = [
    [1, "pages", "Destack launch", "docs", "/launch"],
    [2, "tasks", "Launch tasks", "tasks", "/tasks"],
    [3, "chat", "# launch", "chat", "/chat"],
    [4, "source", "destack", "code", "/code"],
    [5, "pages", "Notes", "site", "/"],
    [6, "calendar", "October 20 – 24", "calendar", "/book"],
    [7, "table", "Signups", "waitlist", "/waitlist"],
    [8, "mail", "Destack opens Friday", "mail", "/mail"],
];

/** The apps of your space on its rail, by the page each opens. */
const rail: readonly (readonly [icon: string, label: string])[] = [
    ["pages", "Pages"],
    ["tasks", "Tasks"],
    ["chat", "Chat"],
    ["source", "Forge"],
    ["calendar", "Calendar"],
    ["table", "Tables"],
    ["mail", "Mail"],
];

/** The height in pixels of one x-ray tag, which sets how the tag column stacks them. */
const TAG_HEIGHT = 24;

/** The gap in pixels between two tags in the tag column. */
const TAG_GAP = 6;

/** Where a part of the page sits in the browser, and the ledger row of the service behind it. */
type Box = { index: number; x: number; y: number; width: number; height: number };

/** The column the tags line up in, or none on a narrow browser, where tags sit on their marks. */
type Column = { left: number; top: number; bottom: number } | undefined;

/** Draw the docs app in a browser, x-rayed: each part outlined with the service that runs it. */
export function XRay(properties: {
    isOpenAt: Stagger;
    lit: number | undefined;
    onLight: (index: number | undefined) => void;
}) {
    // hold the browser, the elements marked for the x-ray and the tag column
    let root: HTMLDivElement | undefined;
    let gutter: HTMLDivElement | undefined;
    const [boxes, setBoxes] = createSignal<readonly Box[]>([]);
    const [column, setColumn] = createSignal<Column>(undefined);

    // hold the app the browser shows, by its page
    const [row, setRow] = createSignal(1);
    const app = () => appOf(row());

    // measure every marked part and the tag column against the browser
    const measure = () => {
        if (!root || !gutter) {
            return;
        }
        const bounds = root.getBoundingClientRect();
        const marked = [...root.querySelectorAll<HTMLElement>("[data-xray]")];
        const measured = marked.map((element) => {
            const rectangle = element.getBoundingClientRect();
            const page = element.closest("main")?.getBoundingClientRect();

            return {
                index: indexOf(element.dataset["xray"] ?? ""),
                x: rectangle.left - bounds.left,
                y: rectangle.top - bounds.top,
                width: rectangle.width,
                height: rectangle.height,
                isHidden:
                    page !== undefined && rectangle.top + rectangle.height / 2 > page.bottom - 16,
            };
        });

        // keep the elements the page shows, leaving out those below its fold
        setBoxes(
            measured
                .filter((box) => !box.isHidden)
                .map(({ index, x, y, width, height }) => ({ index, x, y, width, height })),
        );

        // keep the column only while it shows
        const lane = gutter.getBoundingClientRect();
        setColumn(
            lane.width === 0
                ? undefined
                : {
                      left: lane.left - bounds.left,
                      top: lane.top - bounds.top,
                      bottom: lane.bottom - bounds.top,
                  },
        );
    };

    // measure again as the browser resizes and once the space takes over the page
    const resizes = makeResizeObserver(measure);
    createEffect(
        () => [properties.isOpenAt(0), row()],
        () => {
            requestAnimationFrame(measure);
        },
    );
    onSettled(() => {
        if (root) {
            resizes.observe(root);
        }
        measure();
    });

    // stack the tags down the column in page order, each as near its part as the one above allows
    const tops = () => {
        // place nothing while the column is hidden
        const lane = column();
        const placed = new Map<number, number>();
        if (lane === undefined) {
            return placed;
        }

        // place each tag at its part's height, or just below the tag above
        let floor = lane.top + TAG_GAP;
        const ordered = boxes().toSorted((first, second) => centreOf(first) - centreOf(second));
        for (const box of ordered) {
            const top = Math.max(centreOf(box) - TAG_HEIGHT / 2, floor);
            placed.set(box.index, top);
            floor = top + TAG_HEIGHT + TAG_GAP;
        }

        return placed;
    };

    // keep a tab for each service still rented, its vendor's dashboard
    const rented = () => services.filter((_, index) => !properties.isOpenAt(index + 1));

    // follow the space once it takes over, and the host once hosting moves into it
    const isOwned = () => properties.isOpenAt(0);
    const isStacked = () => !properties.isOpenAt(0);
    const isHosted = () => properties.isOpenAt(9);

    return (
        <div ref={root} data-component="XRay" {...style.attrs(browserStyles.browser, styles.root)}>
            {/* Tabs */}
            <div data-component="TabStrip" {...style.attrs(browserStyles.strip)}>
                <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                {isOwned() ? (
                    <span
                        data-component="Tab"
                        {...style.attrs(browserStyles.tab, styles.tabWide, browserStyles.tabOn)}
                    >
                        <Favicon icon="user" tint={palette.teal} />
                        <span {...style.attrs(browserStyles.tabTitle)}>Florian</span>
                        <Glyph name="close" size={12} />
                    </span>
                ) : (
                    apps.map(([page, , title]) => (
                        <span
                            data-component="Tab"
                            title={title}
                            onClick={() => setRow(page)}
                            {...style.attrs(
                                browserStyles.tab,
                                page === row() ? styles.tabWide : browserStyles.tabEven,
                                page === row() && browserStyles.tabOn,
                            )}
                        >
                            <span {...style.attrs(styles.monogram)}>S</span>
                            <span
                                {...style.attrs(
                                    browserStyles.tabTitle,
                                    page === row() ? styles.shown : browserStyles.tabTitleHidden,
                                )}
                            >
                                {title}
                            </span>
                        </span>
                    ))
                )}
                {rented().map((service) => (
                    <span
                        data-component="Tab"
                        title={`${service.stacked.name} dashboard`}
                        {...style.attrs(browserStyles.tab, browserStyles.tabEven)}
                    >
                        <img
                            alt=""
                            src={logoOf(service.stacked.mark)}
                            {...style.attrs(styles.favicon)}
                        />
                    </span>
                ))}
                <span {...style.attrs(browserStyles.newTab)}>
                    <Glyph name="plus" />
                </span>
            </div>

            {/* Address bar */}
            <div data-component="Toolbar" {...style.attrs(browserStyles.toolbar)}>
                <span {...style.attrs(browserStyles.tools, browserStyles.desktopOnly)}>
                    <Glyph name="back" />
                    <Glyph name="forward" />
                    <Glyph name="reload" />
                </span>
                <span data-component="AddressBar" {...style.attrs(browserStyles.omnibox)}>
                    <Glyph name="lock" size={12} />
                    <span data-xray="Hosting" {...style.attrs(browserStyles.url, styles.url)}>
                        <span {...style.attrs(browserStyles.host)}>
                            {isHosted() ? "destack.app" : `${app()[3]}.symbol.dev`}
                        </span>
                        <span {...style.attrs(browserStyles.path)}>
                            {isHosted() ? `/@florian${pathOf(app())}` : app()[4]}
                        </span>
                    </span>
                    <span {...style.attrs(styles.push)} />
                    <Glyph name="star" />
                </span>
                <span {...style.attrs(browserStyles.tabCount)}>
                    {(isOwned() ? 1 : apps.length) + rented().length}
                </span>
                <span {...style.attrs(browserStyles.tools)}>
                    <span {...style.attrs(browserStyles.profile, browserStyles.desktopOnly)}>
                        F
                    </span>
                    <Glyph name="menu" />
                </span>
            </div>

            {/* Page, with the tag column beside it */}
            <div {...style.attrs(browserStyles.viewport, styles.viewport)}>
                <Fade
                    isOpen={isOwned()}
                    isLarge
                    xstyle={[appStyles.app, styles.app, isStacked() && styles.bare]}
                >
                    {isOwned() ? (
                        <AppRail
                            open={app()[1]}
                            apps={rail}
                            onOpen={(icon) => setRow(rowOfApp(icon))}
                        />
                    ) : undefined}
                    <main {...style.attrs(appStyles.main, styles.page)}>{sectionOf(row())}</main>
                    <p {...style.attrs(appStyles.statusBar)}>
                        <span {...style.attrs(appStyles.status)}>
                            <span {...style.attrs(appStyles.dot)} />
                            {isOwned()
                                ? `Yours · @florian${pathOf(app())}`
                                : `${app()[3]}.symbol.dev`}
                        </span>
                        <span data-xray="Code" {...style.attrs(appStyles.statusEnd, styles.commit)}>
                            <Icon icon={gitCommit} />
                            <span {...style.attrs(styles.code)}>a1f3c9e</span>
                            deployed 2h ago
                        </span>
                    </p>
                </Fade>
                <div ref={gutter} aria-hidden="true" {...style.attrs(styles.gutter)} />
            </div>

            {/* X-ray */}
            <div aria-hidden="true" {...style.attrs(styles.overlay)}>
                <svg {...style.attrs(styles.leaders)}>
                    {boxes().map((box) => (
                        <Leader
                            box={box}
                            column={column()}
                            top={tops().get(box.index) ?? 0}
                            accent={accentAt(properties.isOpenAt, box.index)}
                            isOwned={properties.isOpenAt(box.index + 1)}
                            isLit={box.index === properties.lit}
                            isDimmed={isDimmed(properties.lit, box.index)}
                        />
                    ))}
                </svg>
                {boxes().map((box) => (
                    <Mark
                        box={box}
                        column={column()}
                        top={tops().get(box.index) ?? 0}
                        accent={accentAt(properties.isOpenAt, box.index)}
                        isOwned={properties.isOpenAt(box.index + 1)}
                        isLit={box.index === properties.lit}
                        isDimmed={isDimmed(properties.lit, box.index)}
                        onLight={properties.onLight}
                    />
                ))}
            </div>
        </div>
    );
}

/** Draw the line from an x-rayed part to its tag in the column. */
function Leader(properties: {
    box: Box;
    column: Column;
    top: number;
    accent: string;
    isOwned: boolean;
    isLit: boolean;
    isDimmed: boolean;
}) {
    // run from the part's right edge to the tag's left, easing across the gap
    const path = () => {
        // draw no line while the column is hidden
        const lane = properties.column;
        if (lane === undefined) {
            return "";
        }

        // bend the line halfway between the part and the tag
        const startX = properties.box.x + properties.box.width + 3;
        const startY = centreOf(properties.box);
        const endX = lane.left + 10;
        const endY = properties.top + TAG_HEIGHT / 2;
        const bend = Math.max((endX - startX) / 2, 12);

        return `M${startX} ${startY}C${startX + bend} ${startY} ${endX - bend} ${endY} ${endX} ${endY}`;
    };

    return (
        <g
            {...style.attrs(
                styles.leader,
                properties.isLit && styles.leaderLit,
                properties.isDimmed && styles.markDimmed,
            )}
        >
            <path
                d={path()}
                fill="none"
                stroke-dasharray={properties.isOwned ? "none" : "3 3"}
                {...style.attrs(styles.leaderLine(properties.accent))}
            />
            <circle
                cx={properties.box.x + properties.box.width + 3}
                cy={centreOf(properties.box)}
                r="2.5"
                {...style.attrs(styles.leaderEnd(properties.accent))}
            />
        </g>
    );
}

/** Draw one x-rayed part: its outline, and its tag in the column or pinned to its corner. */
function Mark(properties: {
    box: Box;
    column: Column;
    top: number;
    accent: string;
    isOwned: boolean;
    isLit: boolean;
    isDimmed: boolean;
    onLight: (index: number | undefined) => void;
}) {
    // set the tag in the column, or pin it to the part's corner on a narrow browser
    const service = () => serviceAt(properties.box.index);
    const isPinned = () => properties.column === undefined;
    const tagLeft = () =>
        properties.column === undefined
            ? properties.box.x + properties.box.width - 12
            : properties.column.left + 10;
    const tagTop = () => (properties.column === undefined ? properties.box.y - 12 : properties.top);

    return (
        <>
            <div
                data-component="XRayMark"
                onPointerEnter={() => properties.onLight(properties.box.index)}
                onPointerLeave={() => properties.onLight(undefined)}
                {...style.attributes(
                    [
                        styles.mark,
                        properties.isOwned
                            ? styles.markOwned(properties.accent)
                            : styles.markRented(properties.accent),
                        properties.isLit && styles.markLit(properties.accent),
                        properties.isDimmed && styles.markDimmed,
                    ],
                    {
                        left: `${properties.box.x - 3}px`,
                        top: `${properties.box.y - 3}px`,
                        width: `${properties.box.width + 6}px`,
                        height: `${properties.box.height + 6}px`,
                    },
                )}
            />
            <span
                data-component="XRayTag"
                onPointerEnter={() => properties.onLight(properties.box.index)}
                onPointerLeave={() => properties.onLight(undefined)}
                {...style.attributes(
                    [
                        styles.tag,
                        properties.isOwned ? styles.tagOwned : styles.tagRented(properties.accent),
                        isPinned() && styles.tagPinned,
                        properties.isLit && styles.tagLit,
                        properties.isDimmed && styles.markDimmed,
                    ],
                    { left: `${tagLeft()}px`, top: `${tagTop()}px` },
                )}
            >
                {properties.isOwned ? (
                    <>
                        <ServiceGlyph icon={iconOf(service().destacked.mark)} />
                        <span {...style.attrs(styles.tagName, isPinned() && styles.hidden)}>
                            {service().destacked.name}
                        </span>
                    </>
                ) : (
                    <>
                        <img
                            alt=""
                            src={logoOf(service().stacked.mark)}
                            {...style.attrs(styles.tagLogo)}
                        />
                        <span {...style.attrs(styles.tagName, isPinned() && styles.hidden)}>
                            {service().stacked.name}
                        </span>
                        <span {...style.attrs(styles.tagWiring, isPinned() && styles.hidden)}>
                            {wiringAt(properties.box.index)}
                        </span>
                    </>
                )}
            </span>
        </>
    );
}

/** Return the app that shows a page. */
function appOf(row: number): (typeof apps)[number] {
    const app = apps.find(([page]) => page === row);
    if (app === undefined) {
        throw new RangeError(`no app shows page ${String(row)}`);
    }

    return app;
}

/** Return the page an app of the rail opens. */
function rowOfApp(icon: string): number {
    return appOf(apps.find(([, glyph]) => glyph === icon)?.[0] ?? 0)[0];
}

/** Return an app's path within your space, where the root page drops its slash. */
function pathOf(app: (typeof apps)[number]): string {
    return app[4] === "/" ? "" : app[4];
}

/** Return the ledger row of the service a part names by its label. */
function indexOf(label: string): number {
    const index = services.findIndex((service) => service.stacked.label === label);
    if (index === -1) {
        throw new RangeError(`no service labelled ${label}`);
    }

    return index;
}

/** Report whether a part fades back while another one is lit. */
function isDimmed(lit: number | undefined, index: number): boolean {
    return lit !== undefined && lit !== index;
}

/** Return the height in the browser a part's middle sits at. */
function centreOf(box: Box): number {
    return box.y + box.height / 2;
}

/** Return the colour a part wears: the engine's orange once owned, or its vendor's accent. */
function accentAt(isOpenAt: Stagger, index: number): string {
    return isOpenAt(index + 1)
        ? palette.ember
        : accentOf(serviceAt(index).stacked.name, color.mutedForeground);
}

/** Return the service on a ledger row, or refuse a row the ledger lacks. */
function serviceAt(index: number): (typeof services)[number] {
    const service = services[index];
    if (service === undefined) {
        throw new RangeError(`no service at ${String(index)}`);
    }

    return service;
}

/** Return how the rented service on a ledger row is wired in. */
function wiringAt(index: number): string {
    const kind = wiring[index];
    if (kind === undefined) {
        throw new RangeError(`no wiring at ${String(index)}`);
    }

    return kind;
}

/** Return the logo file of a vendor's mark. */
function logoOf(mark: (typeof services)[number]["stacked"]["mark"]): string {
    if (!("logo" in mark)) {
        throw new TypeError("rented service without a logo");
    }

    return `/logos/${mark.logo}`;
}

/** Return the glyph of an app's mark. */
function iconOf(mark: (typeof services)[number]["destacked"]["mark"]): string {
    if (!("app" in mark)) {
        throw new TypeError("owned service without an app mark");
    }

    return mark.app;
}

/** The x-ray styles. */
const styles = style.create({
    root: {
        containerType: "inline-size",
        position: "relative",
    },
    tabWide: {
        flexBasis: "11rem",
        flexShrink: 1,
        paddingInline: "0.75rem",
    },
    shown: {
        display: "block",
    },
    url: {
        flexGrow: 0,
        flexShrink: 1,
    },
    push: {
        flexGrow: 1,
    },
    monogram: {
        alignItems: "center",
        backgroundColor: palette.ink,
        borderRadius: "4px",
        color: palette.tileGlyph,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "0.5625rem",
        fontWeight: 800,
        height: "14px",
        justifyContent: "center",
        width: "14px",
    },
    favicon: {
        borderRadius: "3px",
        flexShrink: 0,
        height: "14px",
        minWidth: "14px",
        width: "14px",
    },
    viewport: {
        position: "relative",
    },
    app: {
        bottom: 0,
        left: 0,
        position: "absolute",
        right: { default: "13rem", [PINNED]: 0 },
        top: 0,
    },
    bare: {
        gridTemplateColumns: "minmax(0, 1fr)",
    },
    gutter: {
        backgroundColor: `color-mix(in srgb, ${color.muted} 60%, transparent)`,
        borderLeftColor: color.border,
        borderLeftStyle: "solid",
        borderLeftWidth: stroke.border,
        bottom: 0,
        display: { default: "block", [PINNED]: "none" },
        position: "absolute",
        right: 0,
        top: 0,
        width: "13rem",
    },
    page: {
        filter: "saturate(0.6)",
    },
    code: {
        fontFamily: font.code,
    },
    commit: {
        gap: "0.375rem",
    },
    leaders: {
        height: "100%",
        inset: 0,
        overflow: "visible",
        position: "absolute",
        width: "100%",
    },
    leader: {
        transitionDuration: "200ms",
        transitionProperty: "opacity",
    },
    leaderLit: {
        strokeWidth: 1.5,
    },
    leaderLine: (accent: string) => ({
        stroke: `color-mix(in srgb, ${accent} 65%, transparent)`,
        strokeWidth: 1,
    }),
    leaderEnd: (accent: string) => ({
        fill: accent,
    }),
    overlay: {
        inset: 0,
        pointerEvents: "none",
        position: "absolute",
    },
    mark: {
        borderRadius: "6px",
        borderWidth: "1.5px",
        boxSizing: "border-box",
        pointerEvents: "auto",
        position: "absolute",
        transitionDuration: "320ms",
        transitionProperty: "opacity, background-color, border-color, left, top, width, height",
        transitionTimingFunction: "ease",
    },
    markRented: (accent: string) => ({
        backgroundColor: `color-mix(in srgb, ${accent} 7%, transparent)`,
        borderColor: `color-mix(in srgb, ${accent} 70%, transparent)`,
        borderStyle: "dashed",
    }),
    markOwned: (accent: string) => ({
        backgroundColor: `color-mix(in srgb, ${accent} 7%, transparent)`,
        borderColor: `color-mix(in srgb, ${accent} 60%, transparent)`,
        borderStyle: "solid",
    }),
    markLit: (accent: string) => ({
        backgroundColor: `color-mix(in srgb, ${accent} 16%, transparent)`,
        borderColor: accent,
    }),
    markDimmed: {
        opacity: 0.3,
    },
    tag: {
        alignItems: "center",
        borderRadius: "5px",
        boxShadow: `0 1px 2px color-mix(in srgb, ${palette.ink} 14%, transparent)`,
        display: "inline-flex",
        fontSize: "0.6875rem",
        fontWeight: 600,
        gap: "0.3125rem",
        height: "24px",
        boxSizing: "border-box",
        lineHeight: 1,
        pointerEvents: "auto",
        paddingBlock: "0.1875rem",
        paddingInlineEnd: "0.375rem",
        paddingInlineStart: "0.1875rem",
        position: "absolute",
        transitionDuration: "320ms",
        transitionProperty: "opacity, left, top",
        transitionTimingFunction: "ease",
        whiteSpace: "nowrap",
    },
    tagPinned: {
        paddingInlineEnd: "0.1875rem",
    },
    tagLit: {
        zIndex: 1,
    },
    hidden: {
        display: "none",
    },
    tagRented: (accent: string) => ({
        backgroundColor: color.card,
        boxShadow: `inset 0 0 0 1px ${accent}, 0 1px 2px color-mix(in srgb, ${palette.ink} 14%, transparent)`,
        color: color.cardForeground,
    }),
    tagOwned: {
        backgroundColor: color.card,
        boxShadow: `inset 0 0 0 1px color-mix(in srgb, ${palette.ember} 45%, transparent), 0 1px 2px color-mix(in srgb, ${palette.ink} 14%, transparent)`,
        color: color.cardForeground,
    },
    tagLogo: {
        backgroundColor: palette.logoGround,
        borderRadius: "3px",
        height: "1rem",
        width: "1rem",
    },
    tagName: {
        display: "inline",
    },
    tagWiring: {
        color: color.mutedForeground,
        display: "inline",
        fontFamily: font.code,
        fontSize: "0.625rem",
        fontWeight: 400,
    },
});
