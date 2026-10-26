import { Icon } from "@destack/icon";
import gitCommit from "@destack/icon/phosphor/git-commit";
import * as style from "@destack/style";
import { color, font, motion } from "@destack/theme/tokens.stylex";
import { createEffect, createSignal, onSettled, useEnvironment } from "@destack/view";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";
import { makeResizeObserver } from "@destack/view/primitives/resize-observer";

import { palette } from "../../palette.stylex";
import { AppRail, appStyles } from "../figure/app";
import { browserStyles } from "../figure/browser";
import { Fade } from "../figure/fade";
import { Favicon, Glyph } from "../figure/glyph";
import { accentOf } from "../figure/item";
import type { Stagger } from "../figure/stagger";
import { sectionOf } from "../verb/apps";
import type { Job } from "../verb/space";
import { sources } from "../verb/source";
import { present } from "@destack/schema";
import { type Service, serviceCount, serviceLabels, serviceParts } from "./service";
import type { Item } from "../figure/item";
import type { OptionMark } from "../figure/picker";
import { typeScale } from "../../layout/type.stylex";
import { requestFrame } from "@destack/view/primitives/raf";

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

/** Your apps on the rail once owned, the launch tracker first. */
const apps: readonly (readonly [
    job: Job,
    icon: string,
    title: string,
    slug: string,
    path: string,
])[] = [
    ["tracker", "telemetry", "Launch tracker", "launch-tracker", "/launch-tracker"],
    ["documents", "pages", "Destack launch", "documents", "/launch"],
    ["tables", "table", "Press", "press", "/press"],
    ["tasks", "tasks", "Launch tasks", "tasks", "/tasks"],
    ["calendar", "calendar", "October 20 – 24", "calendar", "/book"],
    ["chat", "chat", "# launch", "chat", "/chat"],
    ["mail", "mail", "Destack opens Friday", "mail", "/mail"],
    ["canvas", "canvas", "Launch plan", "plan", "/plan"],
    ["workflows", "sync", "Workflows", "workflows", "/workflows"],
    ["builder", "build", "Signups", "waitlist", "/waitlist"],
];

/** The apps of your space on its rail, by the page each opens. */
const rail: readonly (readonly [icon: string, label: string])[] = [
    ["pages", "Pages"],
    ["table", "Tables"],
    ["tasks", "Tasks"],
    ["calendar", "Calendar"],
    ["chat", "Chat"],
    ["mail", "Mail"],
    ["canvas", "Canvas"],
    ["sync", "Workflows"],
    ["build", "Waitlist"],
    ["telemetry", "Launch tracker"],
];

/** The space in pixels every outline leaves around the element it marks. */
const PAD = 4;

/** The height in pixels of a tag, which sits on its outline's top edge, just above it. */
const TAG_HEIGHT = 20;

/** How far in pixels a tag sits from its outline's left corner. */
const TAG_INSET = 8;

/** The width in pixels a tag takes per character of its text, to estimate its size. */
const TAG_CHARACTER = 6.2;

/** The width in pixels a tag takes with its mark alone. */
const TAG_MARK = 20;

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

/** Draw your launch tracker in a browser, x-rayed: each part outlined with its service. */
export function XRay(properties: {
    rented: readonly Service[];
    isOpenAt: Stagger;
    lit: number | undefined;
    onLight: (row: number | undefined) => void;
}) {
    // hold the page's display, the browser and the elements marked for the x-ray
    const environment = useEnvironment();
    const [root, setRoot] = createSignal<HTMLDivElement>();
    const [boxes, setBoxes] = createSignal<readonly Box[]>([]);

    // hold the app the browser shows, by its page
    const [row, setRow] = createSignal<Job>("tracker");
    const app = () => appOf(row());

    // measure every marked element against the browser, only while it shows
    const isShown = createVisibilityObserver(root, { initialValue: false });
    const measure = () => {
        // require the browser on screen
        const browser = root();
        if (!browser || !isShown()) {
            return;
        }

        // measure each marked element the page shows above its fold
        const bounds = browser.getBoundingClientRect();
        const marked = [...browser.querySelectorAll<HTMLElement>("[data-xray]")];
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
        const obstacles = textOf(browser, bounds);
        const placed: Rectangle[] = [];
        setBoxes(
            measured.map(({ index, source, x, y, width, height }) => {
                // find the tag's spot, and keep later tags clear of it
                const tagWidth = tagWidthOf(
                    present(properties.rented[index], "rented service"),
                    index,
                    source,
                    properties.isOpenAt(index + 1),
                    index + 1 === properties.lit,
                );
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

    // measure again as the browser resizes, comes on screen, and once the space takes over the page
    const resizes = makeResizeObserver(measure);
    createEffect(
        () => [
            isShown(),
            row(),
            properties.rented,
            properties.lit,
            ...Array.from({ length: serviceCount }, (_, index) => properties.isOpenAt(index)),
        ],
        () => requestFrame(measure, environment.display),
    );
    onSettled(() => {
        const browser = root();
        if (browser) {
            resizes.observe(browser);
        }
        measure();
    });

    // keep a tab for each service still rented, its vendor's dashboard
    const rented = () => properties.rented.filter((_, index) => !properties.isOpenAt(index + 1));

    // follow the space once it takes over, and the host once hosting moves into it
    const isOwned = () => properties.isOpenAt(0);
    const isStacked = () => !properties.isOpenAt(0);
    const isHosted = () => properties.isOpenAt(9);

    return (
        <div
            ref={setRoot}
            data-component="XRay"
            data-morph="xray"
            {...style.attrs(browserStyles.browser, styles.root)}
        >
            {/* Tabs */}
            <div data-component="TabStrip" data-morph-strip {...style.attrs(browserStyles.strip)}>
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
                    apps.slice(0, 1).map(([page, , title]) => (
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
                            <Favicon icon="computer" tint={palette.ink} />
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
                        title={`${service.name} dashboard`}
                        {...style.attrs(browserStyles.tab, browserStyles.tabEven)}
                    >
                        <VendorMark mark={service.mark} isTag={false} />
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
                            {isHosted() ? "destack.app" : "localhost:5173"}
                        </span>
                        <span {...style.attrs(browserStyles.path)}>
                            {isHosted() ? `/@florian${pathOf(app())}` : "/"}
                        </span>
                    </span>
                    <span {...style.attrs(styles.push)} />
                    <Glyph name="star" />
                </span>
                <span {...style.attrs(browserStyles.tabCount)}>{1 + rented().length}</span>
                <span {...style.attrs(browserStyles.tools)}>
                    <span {...style.attrs(browserStyles.profile, browserStyles.desktopOnly)}>
                        F
                    </span>
                    <Glyph name="menu" />
                </span>
            </div>

            {/* Page */}
            <div data-morph-page {...style.attrs(browserStyles.viewport, styles.viewport)}>
                <Fade
                    isOpen={isOwned()}
                    isLarge
                    xstyle={[appStyles.app, styles.app, isStacked() && styles.bare]}
                >
                    {isOwned() ? (
                        <AppRail
                            open={app()[1]}
                            apps={rail}
                            morph={undefined}
                            onOpen={(icon) => setRow(rowOfApp(icon))}
                        />
                    ) : undefined}
                    <main {...style.attrs(appStyles.main, styles.page)}>
                        {sectionOf(row(), "start", present(sources[0], "first source"), false)}
                    </main>
                    <p {...style.attrs(appStyles.statusBar)}>
                        <span data-xray="Code" {...style.attrs(appStyles.statusEnd, styles.commit)}>
                            <Icon icon={gitCommit} />
                            <span {...style.attrs(styles.code)}>a1f3c9e</span>· 2h ago
                        </span>
                    </p>
                </Fade>
            </div>

            {/* X-ray */}
            <div aria-hidden="true" data-morph-tags {...style.attrs(styles.overlay)}>
                {boxes().map((box) => (
                    <Mark
                        box={box}
                        service={present(properties.rented[box.index], "rented service")}
                        accent={accentAt(properties.rented, properties.isOpenAt, box.index)}
                        isOwned={properties.isOpenAt(box.index + 1)}
                        isLit={box.index + 1 === properties.lit}
                        onLight={properties.onLight}
                    />
                ))}
            </div>
        </div>
    );
}

/** Draw one x-rayed part: its outline, and a tag naming its service and source app. */
function Mark(properties: {
    service: Service;
    box: Box;
    accent: string;
    isOwned: boolean;
    isLit: boolean;
    onLight: (row: number | undefined) => void;
}) {
    // set the tag where the measure placed it, on a corner of the outline
    const part = () => present(serviceParts[properties.box.index], "engine part");
    const tagLeft = () => properties.box.tagX;
    const tagTop = () => properties.box.tagY;

    return (
        <>
            <div
                data-component="XRayMark"
                onPointerEnter={() => properties.onLight(properties.box.index + 1)}
                onPointerLeave={() => properties.onLight(undefined)}
                {...style.attributes(
                    [
                        styles.mark,
                        properties.isOwned
                            ? styles.markOwned(properties.accent)
                            : styles.markRented(properties.accent),
                        properties.isLit ? styles.markLit(properties.accent) : styles.markQuiet,
                    ],
                    {
                        transform: `translate(${properties.box.x - PAD}px, ${properties.box.y - PAD}px)`,
                        width: `${properties.box.width + PAD * 2}px`,
                        height: `${properties.box.height + PAD * 2}px`,
                    },
                )}
            />
            <span
                data-component="XRayTag"
                onPointerEnter={() => properties.onLight(properties.box.index + 1)}
                onPointerLeave={() => properties.onLight(undefined)}
                {...style.attributes(
                    [
                        styles.tag,
                        properties.isOwned ? styles.tagOwned : styles.tagRented(properties.accent),
                        properties.isLit ? styles.tagLit : styles.tagQuiet,
                    ],
                    { transform: `translate(${tagLeft()}px, ${tagTop()}px)` },
                )}
            >
                {properties.isOwned ? (
                    <>
                        <span
                            style={{
                                "mask-image": `url(/diagram/${iconOf(part().mark)}.svg)`,
                            }}
                            {...style.attrs(styles.tagGlyph)}
                        />
                        <span
                            {...style.attrs(styles.tagName, !properties.isLit && styles.tagHidden)}
                        >
                            {part().name}
                        </span>
                        {properties.box.source === undefined ? undefined : (
                            <span
                                {...style.attrs(
                                    styles.tagWiring,
                                    !properties.isLit && styles.tagHidden,
                                )}
                            >
                                {properties.box.source}
                            </span>
                        )}
                    </>
                ) : (
                    <>
                        <VendorMark mark={properties.service.mark} isTag />
                        <span
                            {...style.attrs(styles.tagName, !properties.isLit && styles.tagHidden)}
                        >
                            {properties.service.name}
                        </span>
                        <span
                            {...style.attrs(
                                styles.tagWiring,
                                !properties.isLit && styles.tagHidden,
                            )}
                        >
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
function tagWidthOf(
    service: Service,
    index: number,
    source: string | undefined,
    isOwned: boolean,
    isLit: boolean,
): number {
    // size an unlit tag to its mark alone, and a lit one to its mark and text
    if (!isLit) {
        return TAG_MARK;
    }
    const part = present(serviceParts[index], "engine part");
    const text = isOwned ? `${part.name} ${source ?? ""}` : `${service.name} ${wiringAt(index)}`;

    return TAG_FRAME + text.length * TAG_CHARACTER;
}

/** Return the app that shows a page. */
function appOf(job: Job): (typeof apps)[number] {
    return present(
        apps.find(([page]) => page === job),
        `app for ${job}`,
    );
}

/** Return the page an app of the rail opens. */
function rowOfApp(icon: string): Job {
    return present(
        apps.find(([, glyph]) => glyph === icon),
        `app with glyph ${icon}`,
    )[0];
}

/** Return an app's path within your space, where the root page drops its slash. */
function pathOf(app: (typeof apps)[number]): string {
    return app[4] === "/" ? "" : app[4];
}

/** Return the ledger row of the service a part names by its label. */
function indexOf(label: string): number {
    const index = serviceLabels.indexOf(label);
    if (index === -1) {
        throw new RangeError(`no service labelled ${label}`);
    }

    return index;
}

/** Return the colour a part wears: the engine's orange once owned, or its vendor's accent. */
function accentAt(rented: readonly Service[], isOpenAt: Stagger, index: number): string {
    const service = present(rented[index], "rented service");
    const tint = "tint" in service.mark ? service.mark.tint : color.mutedForeground;

    return isOpenAt(index + 1) ? palette.ember : accentOf(service.name, tint);
}

/** Return how the rented service on a ledger row is wired in. */
function wiringAt(index: number): string {
    const kind = wiring[index];
    if (kind === undefined) {
        throw new RangeError(`no wiring at ${String(index)}`);
    }

    return kind;
}

/** Draw a rented service's mark: its picture, or its shape from the icon set in its tint. */
function VendorMark(properties: { mark: OptionMark; isTag: boolean }) {
    return (
        <>
            {"logo" in properties.mark ? (
                <img
                    alt=""
                    src={`/logos/${properties.mark.logo}`}
                    {...style.attrs(properties.isTag ? styles.tagLogo : styles.favicon)}
                />
            ) : (
                <Favicon
                    icon={properties.mark.icon}
                    tint={properties.isTag ? palette.tileGlyph : properties.mark.tint}
                    size={properties.isTag ? 13 : 16}
                />
            )}
        </>
    );
}

/** Return the glyph of an app's mark. */
function iconOf(mark: Item["mark"]): string {
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
        left: 0,
        pointerEvents: "auto",
        position: "absolute",
        top: 0,
        transitionDuration: motion.durationMedium,
        transitionProperty: "opacity, background-color, border-color, transform",
        transitionTimingFunction: motion.easingStandard,
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
    markQuiet: {
        backgroundColor: "transparent",
        opacity: 0.45,
    },
    tag: {
        alignItems: "center",
        borderRadius: "4px",
        boxShadow: `0 1px 2px color-mix(in srgb, ${palette.ink} 18%, transparent)`,
        display: "inline-flex",
        fontSize: typeScale.label,
        fontWeight: 600,
        gap: "0.25rem",
        height: "20px",
        boxSizing: "border-box",
        lineHeight: 1,
        pointerEvents: "auto",
        paddingBlock: "0.125rem",
        paddingInlineEnd: { default: "0.375rem", [NARROW]: "0.125rem" },
        paddingInlineStart: "0.125rem",
        left: 0,
        position: "absolute",
        top: 0,
        transitionDuration: motion.durationMedium,
        transitionProperty: "opacity, transform",
        transitionTimingFunction: motion.easingStandard,
        whiteSpace: "nowrap",
    },
    tagLit: {
        zIndex: 1,
    },
    tagQuiet: {
        opacity: 0.75,
    },
    tagHidden: {
        display: "none",
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
        fontSize: typeScale.label,
        fontWeight: 400,
    },
});
