import { Icon } from "@destack/icon";
import gitCommit from "@destack/icon/phosphor/git-commit";
import * as style from "@destack/style";
import { color, font } from "@destack/theme/tokens.stylex";
import { createEffect, createSignal, onSettled } from "@destack/view";
import { makeResizeObserver } from "@destack/view/primitives/resize-observer";

import { palette } from "../../palette.stylex";
import { AppRail, appStyles } from "../figure/app";
import { browserStyles } from "../figure/browser";
import { Fade } from "../figure/fade";
import { Favicon, Glyph } from "../figure/glyph";
import { accentOf } from "../figure/item";
import type { Stagger } from "../figure/stagger";
import { sectionOf } from "../verb/apps";
import { services } from "./service";

/** A browser narrower than a phone, where each tag shows only its mark. */
const NARROW = "@container (max-width: 479px)";

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

/** The space in pixels every outline leaves around the element it marks. */
const PAD = 4;

/** The height in pixels of a tag, which sits on its outline's top edge, just above it. */
const TAG_HEIGHT = 20;

/** How far in pixels a tag sits from its outline's left corner. */
const TAG_INSET = 8;

/** The width in pixels a tag takes per character of its text, to estimate its size. */
const TAG_CHARACTER = 6.2;

/** The width in pixels a tag's mark and padding take, beside its text. */
const TAG_FRAME = 30;

/** Where a part of the page sits, its service and source app, and where its tag sits. */
type Box = {
    index: number;
    source: string | undefined;
    x: number;
    y: number;
    width: number;
    height: number;
    tagX: number;
    tagY: number;
};

/** A rectangle in the browser's own coordinates. */
type Rectangle = { left: number; top: number; right: number; bottom: number };

/** Draw the docs app in a browser, x-rayed: each part outlined with the service that runs it. */
export function XRay(properties: {
    isOpenAt: Stagger;
    lit: number | undefined;
    onLight: (index: number | undefined) => void;
}) {
    // hold the browser and the elements marked for the x-ray
    let root: HTMLDivElement | undefined;
    const [boxes, setBoxes] = createSignal<readonly Box[]>([]);

    // hold the app the browser shows, by its page
    const [row, setRow] = createSignal(1);
    const app = () => appOf(row());

    // measure every marked element against the browser
    const measure = () => {
        if (!root) {
            return;
        }
        // measure each marked element the page shows above its fold
        const bounds = root.getBoundingClientRect();
        const marked = [...root.querySelectorAll<HTMLElement>("[data-xray]")];
        const measured = marked
            .map((element) => {
                const rectangle = element.getBoundingClientRect();
                const page = element.closest("main")?.getBoundingClientRect();

                return {
                    index: indexOf(element.dataset["xray"] ?? ""),
                    source: element.dataset["source"],
                    x: rectangle.left - bounds.left,
                    y: rectangle.top - bounds.top,
                    width: rectangle.width,
                    height: rectangle.height,
                    isHidden:
                        page !== undefined &&
                        rectangle.top + rectangle.height / 2 > page.bottom - 16,
                };
            })
            .filter((box) => !box.isHidden);

        // place each tag at the first corner of its outline that covers no text or other tag
        const obstacles = textOf(root, bounds);
        const placed: Rectangle[] = [];
        setBoxes(
            measured.map(({ index, source, x, y, width, height }) => {
                // find the tag's spot, and keep later tags clear of it
                const tagWidth = tagWidthOf(index, source, properties.isOpenAt(index + 1));
                const spot = spotOf({ x, y, width, height }, tagWidth, bounds, [
                    ...obstacles,
                    ...placed,
                ]);
                placed.push({
                    left: spot.x,
                    top: spot.y,
                    right: spot.x + tagWidth,
                    bottom: spot.y + TAG_HEIGHT,
                });

                return { index, source, x, y, width, height, tagX: spot.x, tagY: spot.y };
            }),
        );
    };

    // measure again as the browser resizes and once the space takes over the page
    const resizes = makeResizeObserver(measure);
    createEffect(
        () => [row(), ...services.map((_, index) => properties.isOpenAt(index))],
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

            {/* Page */}
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
                        <span data-xray="Code" {...style.attrs(appStyles.statusEnd, styles.commit)}>
                            <Icon icon={gitCommit} />
                            <span {...style.attrs(styles.code)}>a1f3c9e</span>· 2h ago
                        </span>
                    </p>
                </Fade>
            </div>

            {/* X-ray */}
            <div aria-hidden="true" {...style.attrs(styles.overlay)}>
                {boxes().map((box) => (
                    <Mark
                        box={box}
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

/** Draw one x-rayed part: its outline, and a tag naming its service and source app. */
function Mark(properties: {
    box: Box;
    accent: string;
    isOwned: boolean;
    isLit: boolean;
    isDimmed: boolean;
    onLight: (index: number | undefined) => void;
}) {
    // set the tag where the measure placed it, on a corner of the outline
    const service = () => serviceAt(properties.box.index);
    const tagLeft = () => properties.box.tagX;
    const tagTop = () => properties.box.tagY;

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
                        left: `${properties.box.x - PAD}px`,
                        top: `${properties.box.y - PAD}px`,
                        width: `${properties.box.width + PAD * 2}px`,
                        height: `${properties.box.height + PAD * 2}px`,
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
                        properties.isLit && styles.tagLit,
                        properties.isDimmed && styles.markDimmed,
                    ],
                    { left: `${tagLeft()}px`, top: `${tagTop()}px` },
                )}
            >
                {properties.isOwned ? (
                    <>
                        <span
                            style={{
                                "mask-image": `url(/diagram/${iconOf(service().destacked.mark)}.svg)`,
                            }}
                            {...style.attrs(styles.tagGlyph)}
                        />
                        <span {...style.attrs(styles.tagName)}>{service().destacked.name}</span>
                        {properties.box.source === undefined ? undefined : (
                            <span {...style.attrs(styles.tagWiring)}>{properties.box.source}</span>
                        )}
                    </>
                ) : (
                    <>
                        <img
                            alt=""
                            src={logoOf(service().stacked.mark)}
                            {...style.attrs(styles.tagLogo)}
                        />
                        <span {...style.attrs(styles.tagName)}>{service().stacked.name}</span>
                        <span {...style.attrs(styles.tagWiring)}>
                            {wiringAt(properties.box.index)}
                        </span>
                    </>
                )}
            </span>
        </>
    );
}

/** Return where a tag sits: the first corner of its outline, above or below, clear of obstacles. */
function spotOf(
    box: { x: number; y: number; width: number; height: number },
    tagWidth: number,
    bounds: DOMRect,
    obstacles: readonly Rectangle[],
): { x: number; y: number } {
    // try above left, above right, below left and below right, in that order
    const left = box.x - PAD + TAG_INSET;
    const right = box.x + box.width + PAD - TAG_INSET - tagWidth;
    const above = box.y - PAD - TAG_HEIGHT + 1;
    const below = box.y + box.height + PAD - 1;
    const corners = [
        { x: left, y: above },
        { x: right, y: above },
        { x: left, y: below },
        { x: right, y: below },
    ];

    // fall back to sliding along either edge past each obstacle, nearest first
    const slides = [above, below].flatMap((y) =>
        obstacles
            .map((obstacle) => ({ x: obstacle.right + PAD, y }))
            .filter((spot) => spot.x > left)
            .toSorted((first, second) => first.x - second.x),
    );
    const spots = [...corners, ...slides];

    // keep the first spot inside the browser that overlaps nothing
    const clear = spots.find(
        (spot) =>
            spot.x >= 0 &&
            spot.y >= 0 &&
            spot.x + tagWidth <= bounds.width &&
            spot.y + TAG_HEIGHT <= bounds.height &&
            !obstacles.some((obstacle) =>
                isOverlap(obstacle, {
                    left: spot.x,
                    top: spot.y,
                    right: spot.x + tagWidth,
                    bottom: spot.y + TAG_HEIGHT,
                }),
            ),
    );

    return clear ?? { x: left, y: above };
}

/** Return the rectangles of every line of text the browser shows, outside the x-ray itself. */
function textOf(root: HTMLElement, bounds: DOMRect): readonly Rectangle[] {
    // walk the text of the page and its toolbar, skipping the tags themselves
    const rectangles: Rectangle[] = [];
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    const range = document.createRange();
    for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
        // read each visible line of a text node, relative to the browser
        if (
            (node.textContent ?? "").trim() === "" ||
            node.parentElement?.closest("[aria-hidden]")
        ) {
            continue;
        }
        range.selectNodeContents(node);
        for (const line of range.getClientRects()) {
            rectangles.push({
                left: line.left - bounds.left,
                top: line.top - bounds.top,
                right: line.right - bounds.left,
                bottom: line.bottom - bounds.top,
            });
        }
    }

    return rectangles;
}

/** Report whether two rectangles overlap. */
function isOverlap(first: Rectangle, second: Rectangle): boolean {
    return (
        first.left < second.right &&
        second.left < first.right &&
        first.top < second.bottom &&
        second.top < first.bottom
    );
}

/** Estimate a tag's width from its text: the service, then its source app or its rented wiring. */
function tagWidthOf(index: number, source: string | undefined, isOwned: boolean): number {
    const service = serviceAt(index);
    const text = isOwned
        ? `${service.destacked.name} ${source ?? ""}`
        : `${service.stacked.name} ${wiringAt(index)}`;

    return TAG_FRAME + text.length * TAG_CHARACTER;
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
        right: 0,
        top: 0,
    },
    bare: {
        gridTemplateColumns: "minmax(0, 1fr)",
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
        borderRadius: "4px",
        boxShadow: `0 1px 2px color-mix(in srgb, ${palette.ink} 18%, transparent)`,
        display: "inline-flex",
        fontSize: "0.65rem",
        fontWeight: 600,
        gap: "0.25rem",
        height: "20px",
        boxSizing: "border-box",
        lineHeight: 1,
        pointerEvents: "auto",
        paddingBlock: "0.125rem",
        paddingInlineEnd: { default: "0.375rem", [NARROW]: "0.125rem" },
        paddingInlineStart: "0.125rem",
        position: "absolute",
        transitionDuration: "320ms",
        transitionProperty: "opacity, left, top",
        transitionTimingFunction: "ease",
        whiteSpace: "nowrap",
    },
    tagLit: {
        zIndex: 1,
    },
    tagRented: (accent: string) => ({
        backgroundColor: accent,
        color: palette.tileGlyph,
    }),
    tagOwned: {
        backgroundColor: palette.ember,
        color: palette.tileGlyph,
    },
    tagGlyph: {
        backgroundColor: palette.tileGlyph,
        flexShrink: 0,
        height: "12px",
        marginInline: "2px",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "12px",
    },
    tagLogo: {
        backgroundColor: palette.logoGround,
        borderRadius: "3px",
        height: "14px",
        width: "14px",
    },
    tagName: {
        display: { default: "inline", [NARROW]: "none" },
    },
    tagWiring: {
        color: `color-mix(in srgb, ${palette.tileGlyph} 78%, transparent)`,
        display: { default: "inline", [NARROW]: "none" },
        fontFamily: font.code,
        fontSize: "0.625rem",
        fontWeight: 400,
    },
});
