import { isServer } from "@solidjs/web";
import { type Accessor, createEffect, createSignal, onCleanup, untrack } from "solid-js";
import { makeEventListener } from "./event-listener.ts";
import { access, type MaybeAccessor, TRANSPARENT } from "./utils.ts";

/** The relation of a favicon link. */
export type FaviconRel = "icon" | "shortcut icon" | "apple-touch-icon";

/** Which favicon link to set. */
export type FaviconOptions = {
    /** The link's relation, `icon` by default. */
    readonly rel?: FaviconRel;
};

/** A favicon link's current address, how to change it, and how to put back what was there. */
export type FaviconController = {
    /** The address the link shows. */
    readonly href: string;
    /** Show another address. */
    setHref: (href: string) => void;
    /** Put back the address the link had, or remove the link it made. */
    dispose: () => void;
};

/** How a favicon animation cycles its frames. */
export type FaviconAnimationOptions = FaviconOptions & {
    /** The milliseconds between frames, 200 by default. */
    readonly interval?: number;
    /** Whether it starts cycling right away, true by default. */
    readonly autoplay?: boolean;
    /** Whether it starts over after the last frame, true by default. */
    readonly loop?: boolean;
};

/** A favicon animation's current frame and controls. */
export type FaviconAnimationController = {
    /** The address of the frame shown. */
    readonly href: string;
    /** The index of the frame shown. */
    readonly frame: number;
    /** Whether it cycles. */
    readonly playing: boolean;
    /** Start cycling. */
    play: () => void;
    /** Stop cycling. */
    pause: () => void;
    /** Stop and put back the favicon. */
    dispose: () => void;
};

/** A badge's value: a count, a short text, a dot for true, or no badge for zero, false, empty or undefined. */
export type FaviconBadgeValue = number | string | boolean | undefined;

/** The corner a badge sits in. */
export type FaviconBadgePosition = "top-left" | "top-right" | "bottom-left" | "bottom-right";

/** How a badge looks. */
export type FaviconBadgeOptions = FaviconOptions & {
    /** The badge's fill color. */
    readonly color?: string;
    /** The badge's text color. */
    readonly textColor?: string;
    /** The corner the badge sits in, bottom right by default. */
    readonly position?: FaviconBadgePosition;
    /** The highest count shown, larger ones reading as this count and a plus, 99 by default. */
    readonly max?: number;
    /** The badge's diameter as a share of the icon, 0.6 by default. */
    readonly scale?: number;
};

/** The scheme a favicon follows. */
export type FaviconColorScheme = "light" | "dark";

/** A favicon for each scheme. */
export type FaviconSchemeIcons = {
    /** The favicon for a light scheme. */
    readonly light: string;
    /** The favicon for a dark scheme. */
    readonly dark: string;
};

/** A scheme favicon's controller, with the scheme it shows. */
export type FaviconSchemeController = FaviconController & {
    /** The scheme the favicon shows. */
    readonly scheme: FaviconColorScheme;
};

/** How a progress ring looks. */
export type FaviconProgressOptions = FaviconOptions & {
    /** The color of the ring's unfilled track. */
    readonly trackColor?: string;
    /** The color of the filled arc. */
    readonly color?: string;
    /** The ring's width as a share of the icon, 0.15 by default. */
    readonly thickness?: number;
};

/** The milliseconds each frame of a favicon animation shows, five frames a second. */
const FRAME_INTERVAL = 200;

/** The highest count a badge shows by default, larger ones reading as this count and a plus. */
const BADGE_MAX = 99;

/** A badge's default diameter as a share of the icon. */
const BADGE_SCALE = 0.6;

/** The font size of a badge's single character, as a share of its radius. */
const BADGE_CHARACTER_SIZE = 1.4;

/** The font size of a badge's longer text, as a share of its radius. */
const BADGE_TEXT_SIZE = 1.1;

/** A progress ring's default width as a share of the icon. */
const PROGRESS_THICKNESS = 0.15;

/** A badge's default fill color, a strong red. */
const BADGE_COLOR = "#e11d48";

/** A badge's default text color. */
const BADGE_TEXT_COLOR = "#ffffff";

/** A progress ring's default track color, a faint shade. */
const TRACK_COLOR = "rgba(0, 0, 0, 0.15)";

/** A progress ring's default arc color, an indigo. */
const PROGRESS_COLOR = "#6366f1";

/** The size an icon is drawn at when its image reports none. */
const DEFAULT_ICON_SIZE = 32;

/** The scheme the server renders. */
const SERVER_SCHEME: FaviconColorScheme = "light";

/** The query of a dark scheme. */
const DARK_QUERY = "(prefers-color-scheme: dark)";

/** Show an address in the document's favicon link, until disposed. */
export function makeFavicon(href: string, options: FaviconOptions = {}): FaviconController {
    // show nothing on the server
    if (isServer) {
        return { href, setHref: () => {}, dispose: () => {} };
    }

    return bindFaviconLink(options.rel ?? "icon", href);
}

/** Show a reactive address in the document's favicon link, putting back the previous one on cleanup. */
export function createFavicon(
    href: MaybeAccessor<string>,
    options: FaviconOptions = {},
): Accessor<string> {
    // read the address on the server
    if (isServer) {
        return () => access(href);
    }

    // show the first address now, and each later one
    const favicon = makeFavicon(
        untrack(() => access(href)),
        options,
    );
    const [current, setCurrent] = createSignal(favicon.href, { ownedWrite: true });
    if (typeof href === "function") {
        createEffect(
            href,
            (next) => {
                favicon.setHref(next);
                setCurrent(favicon.href);
            },
            { defer: true, ...TRANSPARENT },
        );
    }
    onCleanup(favicon.dispose);

    return current;
}

/** Cycle the favicon through frames every interval, until disposed. */
export function makeFaviconAnimation(
    frames: readonly string[],
    options: FaviconAnimationOptions = {},
): FaviconAnimationController {
    // cycle nothing on the server
    const { interval = FRAME_INTERVAL, autoplay = true, loop = true, ...faviconOptions } = options;
    const first = frameOf(frames, 0);
    if (isServer) {
        return {
            href: first,
            frame: 0,
            playing: false,
            play: () => {},
            pause: () => {},
            dispose: () => {},
        };
    }

    // show the next frame each interval, stopping after the last unless looping
    const favicon = makeFavicon(first, faviconOptions);
    let frame = 0;
    let handle: ReturnType<typeof setInterval> | undefined;
    const pause = (): void => {
        clearInterval(handle);
        handle = undefined;
    };
    const tick = (): void => {
        const next = nextFrame(frame, frames.length, loop);
        if (next === undefined) {
            pause();
        } else {
            frame = next;
            favicon.setHref(frameOf(frames, next));
        }
    };
    const play = (): void => {
        if (handle === undefined && frames.length > 1) {
            handle = setInterval(tick, interval);
        }
    };
    if (autoplay) {
        play();
    }

    return {
        get href() {
            return favicon.href;
        },
        get frame() {
            return frame;
        },
        get playing() {
            return handle !== undefined;
        },
        play,
        pause,
        dispose: () => {
            pause();
            favicon.dispose();
        },
    };
}

/** Cycle the favicon through reactive frames every interval, pausing while the page is hidden, putting back the favicon on cleanup. */
export function createFaviconAnimation(
    frames: MaybeAccessor<readonly string[]>,
    options: FaviconAnimationOptions = {},
): { frame: Accessor<number>; playing: Accessor<boolean>; play: () => void; pause: () => void } {
    // cycle nothing on the server
    if (isServer) {
        return { frame: () => 0, playing: () => false, play: () => {}, pause: () => {} };
    }
    const { interval = FRAME_INTERVAL, autoplay = true, loop = true, ...faviconOptions } = options;
    const [frame, setFrame] = createSignal(0, { ownedWrite: true });
    const [playing, setPlaying] = createSignal(false, { ownedWrite: true });

    // show the next frame each interval, stopping after the last unless looping
    let list = untrack(() => access(frames));
    const favicon = makeFavicon(frameOf(list, 0), faviconOptions);
    let handle: ReturnType<typeof setInterval> | undefined;
    let current = 0;
    const pause = (): void => {
        setPlaying(false);
        clearInterval(handle);
        handle = undefined;
    };
    const tick = (): void => {
        const next = nextFrame(current, list.length, loop);
        if (next === undefined) {
            pause();
        } else {
            current = next;
            setFrame(next);
            favicon.setHref(frameOf(list, next));
        }
    };
    const play = (): void => {
        if (handle === undefined && list.length > 1) {
            setPlaying(true);
            handle = setInterval(tick, interval);
        }
    };

    // start over with new frames, pausing below two and resuming at two or more when autoplaying
    if (typeof frames === "function") {
        createEffect(
            frames,
            (next) => {
                // show the first of the new frames
                list = next;
                current = 0;
                setFrame(0);
                favicon.setHref(frameOf(list, 0));
                if (list.length <= 1) {
                    pause();
                } else if (autoplay) {
                    play();
                }
            },
            { defer: true, ...TRANSPARENT },
        );
    }

    // pause while the page is hidden, resuming once it shows again if it was playing
    let wasPlaying = false;
    makeEventListener(document, "visibilitychange", () => {
        if (document.hidden) {
            wasPlaying = handle !== undefined;
            pause();
        } else if (wasPlaying) {
            wasPlaying = false;
            play();
        }
    });
    if (autoplay) {
        play();
    }
    onCleanup(() => {
        pause();
        favicon.dispose();
    });

    return { frame, playing, play, pause };
}

/** Draw a badge over a favicon: a count, a short text or a dot, until disposed. */
export function makeFaviconBadge(
    href: string,
    value: FaviconBadgeValue,
    options: FaviconBadgeOptions = {},
): FaviconController {
    // draw nothing on the server
    if (isServer) {
        return { href, setHref: () => {}, dispose: () => {} };
    }

    return guardRender(makeFavicon(href, options), renderBadge(href, value, options));
}

/** Draw a reactive badge over a reactive favicon, applying only the latest drawing, putting back the favicon on cleanup. */
export function createFaviconBadge(
    href: MaybeAccessor<string>,
    value: MaybeAccessor<FaviconBadgeValue>,
    options: FaviconBadgeOptions = {},
): Accessor<string> {
    return createCanvasFavicon(href, value, options, renderBadge);
}

/** Show the favicon of the device's scheme, following it as it changes, until disposed. */
export function makeFaviconScheme(
    icons: FaviconSchemeIcons,
    options: FaviconOptions = {},
): FaviconSchemeController {
    // show the light icon on the server
    if (isServer) {
        return {
            href: icons[SERVER_SCHEME],
            scheme: SERVER_SCHEME,
            setHref: () => {},
            dispose: () => {},
        };
    }

    // show the scheme's icon, again whenever the scheme changes
    const list = window.matchMedia(DARK_QUERY);
    let scheme: FaviconColorScheme = list.matches ? "dark" : "light";
    const favicon = makeFavicon(icons[scheme], options);
    const stop = makeEventListener(list, "change", (event) => {
        scheme = event.matches ? "dark" : "light";
        favicon.setHref(icons[scheme]);
    });

    return {
        get href() {
            return favicon.href;
        },
        get scheme() {
            return scheme;
        },
        setHref: favicon.setHref,
        dispose: () => {
            stop();
            favicon.dispose();
        },
    };
}

/** Show the favicon of the device's scheme from reactive icons, following both, putting back the favicon on cleanup. */
export function createFaviconScheme(
    icons: MaybeAccessor<FaviconSchemeIcons>,
    options: FaviconOptions = {},
): { href: Accessor<string>; scheme: Accessor<FaviconColorScheme> } {
    // show the light icon on the server
    if (isServer) {
        return { href: () => access(icons)[SERVER_SCHEME], scheme: () => SERVER_SCHEME };
    }

    // follow the scheme, and show its icon now
    const list = window.matchMedia(DARK_QUERY);
    const [scheme, setScheme] = createSignal<FaviconColorScheme>(list.matches ? "dark" : "light", {
        ownedWrite: true,
    });
    makeEventListener(list, "change", (event) => setScheme(event.matches ? "dark" : "light"));
    const favicon = makeFavicon(
        untrack(() => access(icons)[scheme()]),
        options,
    );
    const [href, setHref] = createSignal(favicon.href, { ownedWrite: true });

    // show the icon again as the icons or the scheme change
    createEffect(
        () => access(icons)[scheme()],
        (next) => {
            favicon.setHref(next);
            setHref(favicon.href);
        },
        { defer: true, ...TRANSPARENT },
    );
    onCleanup(favicon.dispose);

    return { href, scheme };
}

/** Draw a progress ring from 0 to 100 around a favicon, none while undefined, until disposed. */
export function makeFaviconProgress(
    href: string,
    progress: number | undefined,
    options: FaviconProgressOptions = {},
): FaviconController {
    // draw nothing on the server
    if (isServer) {
        return { href, setHref: () => {}, dispose: () => {} };
    }

    return guardRender(makeFavicon(href, options), renderProgress(href, progress, options));
}

/** Draw a reactive progress ring around a reactive favicon, applying only the latest drawing, putting back the favicon on cleanup. */
export function createFaviconProgress(
    href: MaybeAccessor<string>,
    progress: MaybeAccessor<number | undefined>,
    options: FaviconProgressOptions = {},
): Accessor<string> {
    return createCanvasFavicon(href, progress, options, renderProgress);
}

/** Find the document's favicon link of a relation, or make one, showing an address, and remember what to put back. */
function bindFaviconLink(rel: FaviconRel, href: string): FaviconController {
    // take the existing link, remembering its attribute, or make one
    const existing = document.head.querySelector<HTMLLinkElement>(`link[rel="${rel}"]`);
    const previous = existing?.getAttribute("href") ?? undefined;
    const link = existing ?? document.createElement("link");
    if (existing === null) {
        link.rel = rel;
        document.head.append(link);
    }
    link.href = href;

    return {
        get href() {
            return link.href;
        },
        setHref: (next) => {
            link.href = next;
        },
        dispose: () => {
            if (existing === null) {
                link.remove();
            } else if (previous === undefined) {
                link.removeAttribute("href");
            } else {
                link.href = previous;
            }
        },
    };
}

/** Read a frame of a favicon animation, refusing a list without it. */
function frameOf(frames: readonly string[], index: number): string {
    const frame = frames[index];
    if (frame === undefined) {
        throw new RangeError(`the favicon animation has no frame ${index}`);
    }

    return frame;
}

/** Read the frame after one, starting over when looping, or undefined after the last. */
function nextFrame(frame: number, length: number, loop: boolean): number | undefined {
    if (frame + 1 < length) {
        return frame + 1;
    }

    return loop ? 0 : undefined;
}

/** Load an image and draw it on a canvas of its size, or report why it failed and draw nothing. */
async function drawBase(
    href: string,
): Promise<
    { canvas: HTMLCanvasElement; context: CanvasRenderingContext2D; size: number } | undefined
> {
    // load the image, reporting a failure, which leaves the plain favicon in place
    const image = new Image();
    try {
        await new Promise<void>((resolve, reject) => {
            image.addEventListener("load", () => resolve(), { once: true });
            image.addEventListener(
                "error",
                () => reject(new Error(`favicon image failed to load: ${href}`)),
                { once: true },
            );
            image.src = href;
        });
    } catch (error) {
        reportError(error);

        return undefined;
    }

    // draw it on a canvas of its size
    const size = image.naturalWidth || image.naturalHeight || DEFAULT_ICON_SIZE;
    const canvas = document.createElement("canvas");
    canvas.width = size;
    canvas.height = size;
    const context = canvas.getContext("2d");
    if (context === null) {
        reportError(new TypeError("favicon canvas has no 2d context"));

        return undefined;
    }
    context.clearRect(0, 0, size, size);
    context.drawImage(image, 0, 0, size, size);

    return { canvas, context, size };
}

/** Draw a badge over an icon as a data address, or keep the icon without a badge. */
async function renderBadge(
    href: string,
    value: FaviconBadgeValue,
    options: FaviconBadgeOptions,
): Promise<string> {
    // keep the icon for no badge, or where it cannot be drawn
    const isShown =
        value === true ||
        (typeof value === "number" && value !== 0) ||
        (typeof value === "string" && value !== "");
    const base = isShown ? await drawBase(href) : undefined;
    if (base === undefined) {
        return href;
    }

    // draw the badge's circle in its corner
    const {
        color = BADGE_COLOR,
        textColor = BADGE_TEXT_COLOR,
        position = "bottom-right",
        max = BADGE_MAX,
        scale = BADGE_SCALE,
    } = options;
    const { canvas, context, size } = base;
    const radius = (size * scale) / 2;
    const x = position.endsWith("right") ? size - radius : radius;
    const y = position.startsWith("bottom") ? size - radius : radius;
    context.beginPath();
    context.arc(x, y, radius, 0, Math.PI * 2);
    context.fillStyle = color;
    context.fill();

    // write its count or text, a dot showing none
    const text =
        typeof value === "number"
            ? value > max
                ? `${max}+`
                : `${value}`
            : typeof value === "string"
              ? value
              : undefined;
    if (text !== undefined) {
        context.fillStyle = textColor;
        context.font = `${Math.round(radius * (text.length > 1 ? BADGE_TEXT_SIZE : BADGE_CHARACTER_SIZE))}px sans-serif`;
        context.textAlign = "center";
        context.textBaseline = "middle";
        context.fillText(text, x, y);
    }

    return canvas.toDataURL("image/png");
}

/** Draw a progress ring around an icon as a data address, or keep the icon without a ring. */
async function renderProgress(
    href: string,
    progress: number | undefined,
    options: FaviconProgressOptions,
): Promise<string> {
    // keep the icon for no progress, or where it cannot be drawn
    const base = progress === undefined ? undefined : await drawBase(href);
    if (base === undefined || progress === undefined) {
        return href;
    }

    // draw the track around the icon
    const {
        trackColor = TRACK_COLOR,
        color = PROGRESS_COLOR,
        thickness = PROGRESS_THICKNESS,
    } = options;
    const { canvas, context, size } = base;
    const width = size * thickness;
    const radius = (size - width) / 2;
    context.lineWidth = width;
    context.lineCap = "round";
    context.strokeStyle = trackColor;
    context.beginPath();
    context.arc(size / 2, size / 2, radius, 0, Math.PI * 2);
    context.stroke();

    // draw the filled share from the top, clockwise
    const share = Math.min(100, Math.max(0, progress));
    if (share > 0) {
        const start = -Math.PI / 2;
        context.strokeStyle = color;
        context.beginPath();
        context.arc(size / 2, size / 2, radius, start, start + (Math.PI * 2 * share) / 100);
        context.stroke();
    }

    return canvas.toDataURL("image/png");
}

/** Apply a drawing once it settles, unless the favicon was disposed first. */
function guardRender(favicon: FaviconController, render: Promise<string>): FaviconController {
    let isDisposed = false;
    render.then((drawn) => {
        if (!isDisposed) {
            favicon.setHref(drawn);
        }
    }, reportError);

    return {
        get href() {
            return favicon.href;
        },
        setHref: favicon.setHref,
        dispose: () => {
            isDisposed = true;
            favicon.dispose();
        },
    };
}

/** Draw a reactive overlay over a reactive favicon, applying only the latest drawing, putting back the favicon on cleanup. */
function createCanvasFavicon<Value, Options extends FaviconOptions>(
    href: MaybeAccessor<string>,
    value: MaybeAccessor<Value>,
    options: Options,
    render: (href: string, value: Value, options: Options) => Promise<string>,
): Accessor<string> {
    // read the plain address on the server
    if (isServer) {
        return () => access(href);
    }

    // apply each drawing unless a newer one or the cleanup replaced it
    const favicon = makeFavicon(
        untrack(() => access(href)),
        options,
    );
    const [current, setCurrent] = createSignal(favicon.href, { ownedWrite: true });
    let request = 0;
    const redraw = (address: string, next: Value): void => {
        const id = ++request;
        render(address, next, options).then((drawn) => {
            if (id === request) {
                favicon.setHref(drawn);
                setCurrent(favicon.href);
            }
        }, reportError);
    };

    // redraw whenever a reactive address or value changes, or once for fixed ones
    if (typeof href === "function" || typeof value === "function") {
        createEffect(
            () => ({ address: access(href), next: access(value) }),
            ({ address, next }) => redraw(address, next),
            TRANSPARENT,
        );
    } else {
        redraw(href, value);
    }
    onCleanup(() => {
        request++;
        favicon.dispose();
    });

    return current;
}
