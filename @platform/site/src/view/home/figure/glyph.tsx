import * as style from "@destack/style";

/** A glyph the browser and its pages draw. */
type GlyphName =
    | "back"
    | "forward"
    | "reload"
    | "lock"
    | "star"
    | "close"
    | "plus"
    | "clock"
    | "video"
    | "globe"
    | "folder"
    | "file"
    | "key"
    | "puzzle"
    | "menu"
    | "download"
    | "upload"
    | "calendar"
    | "person"
    | "status"
    | "check"
    | "search"
    | "more"
    | "caret"
    | "share"
    | "help"
    | "copy"
    | "link"
    | "grid"
    | "inbox"
    | "target"
    | "layers"
    | "filter"
    | "sliders"
    | "compose"
    | "mail"
    | "like"
    | "dislike"
    | "microphone"
    | "up"
    | "open";

/** The strokes of each glyph, on a 24-unit grid. */
const glyphs: Record<GlyphName, readonly string[]> = {
    back: ["m15 18-6-6 6-6"],
    forward: ["m9 18 6-6-6-6"],
    reload: ["M21 12a9 9 0 1 1-3-6.7L21 8", "M21 3v5h-5"],
    lock: ["M7 11V7a5 5 0 0 1 10 0v4", "M5 11h14v10H5z"],
    star: ["m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z"],
    close: ["M18 6 6 18", "m6 6 12 12"],
    plus: ["M12 5v14", "M5 12h14"],
    clock: ["M12 6v6l4 2", "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z"],
    video: ["m16 10 5-3v10l-5-3", "M3 6h13v12H3z"],
    globe: [
        "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
        "M3 12h18",
        "M12 3a14 14 0 0 1 0 18a14 14 0 0 1 0-18",
    ],
    folder: ["M3 6a1 1 0 0 1 1-1h5l2 2h9a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z"],
    file: ["M14 3H6a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8z", "M14 3v5h5"],
    key: ["M15 7a4 4 0 1 1-3.5 6L5 19.5V16h3v-3h2.5", "M16 8h.01"],
    puzzle: [
        "M10 4a2 2 0 1 1 4 0v1h4v4h1a2 2 0 1 1 0 4h-1v5h-5v-1a2 2 0 1 0-4 0v1H5v-5h1a2 2 0 1 0 0-4H5V5h5z",
    ],
    menu: ["M12 5h.01", "M12 12h.01", "M12 19h.01"],
    download: ["M12 4v11", "m7 10 5 5 5-5", "M5 20h14"],
    upload: ["M12 20V9", "m7 14 5-5 5 5", "M5 4h14"],
    calendar: ["M4 6h16v14H4z", "M4 10h16", "M8 3v4", "M16 3v4"],
    person: ["M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8z", "M4 21a8 8 0 0 1 16 0"],
    status: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 7v5h4"],
    check: ["m5 12 5 5 9-10"],
    search: ["M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14z", "m20 20-4-4"],
    more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
    caret: ["m6 9 6 6 6-6"],
    share: ["M12 3v12", "m7 8 5-5 5 5", "M5 14v6h14v-6"],
    help: [
        "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
        "M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.6",
        "M12 17h.01",
    ],
    copy: ["M8 8h12v12H8z", "M16 8V4H4v12h4"],
    link: [
        "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1",
        "M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
    ],
    grid: ["M4 4h7v7H4z", "M13 4h7v7h-7z", "M4 13h7v7H4z", "M13 13h7v7h-7z"],
    inbox: ["M3 13h5l1.5 3h5L16 13h5", "M5 5h14l2 8v6H3v-6z"],
    target: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z"],
    layers: ["m12 3 9 5-9 5-9-5z", "m3 13 9 5 9-5"],
    filter: ["M4 6h16", "M7 12h10", "M10 18h4"],
    sliders: ["M4 7h10", "M18 7h2", "M4 17h4", "M12 17h8", "M16 5v4", "M10 15v4"],
    compose: ["M4 20h4L19 9l-4-4L4 16z", "m13.5 6.5 4 4"],
    mail: ["M3 6h18v12H3z", "m3 7 9 6 9-6"],
    like: [
        "M7 10v11",
        "M15 5.9 14 10h5.8a2 2 0 0 1 1.9 2.6l-2.3 8a2 2 0 0 1-1.9 1.4H4a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2h2.8a2 2 0 0 0 1.8-1.1L12 2a3.1 3.1 0 0 1 3 3.9Z",
    ],
    dislike: [
        "M17 14V3",
        "M9 18.1 10 14H4.2a2 2 0 0 1-1.9-2.6l2.3-8A2 2 0 0 1 6.5 2H20a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-2.8a2 2 0 0 0-1.8 1.1L12 22a3.1 3.1 0 0 1-3-3.9Z",
    ],
    microphone: [
        "M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z",
        "M19 10v2a7 7 0 0 1-14 0v-2",
        "M12 19v3",
    ],
    up: ["M12 19V5", "m5 12 7-7 7 7"],
    open: ["M7 17 17 7", "M7 7h10v10"],
};

/** Draw a glyph in the current colour, at a size in pixels. */
export function Glyph(properties: { name: GlyphName; size?: number; weight?: number }) {
    const size = () => `${properties.size ?? 14}px`;

    return (
        <svg
            aria-hidden="true"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width={String(properties.weight ?? 2)}
            stroke-linecap="round"
            stroke-linejoin="round"
            style={{ height: size(), width: size() }}
            {...style.attrs(styles.glyph)}
        >
            {glyphs[properties.name].map((path) => (
                <path d={path} />
            ))}
        </svg>
    );
}

/** Draw a brand or app icon from the diagram set in its colour, at a size in pixels. */
export function Favicon(properties: { icon: string; tint: string; size?: number }) {
    const size = () => `${properties.size ?? 14}px`;

    return (
        <span
            aria-hidden="true"
            style={{
                "mask-image": `url(/diagram/${properties.icon}.svg)`,
                "background-color": properties.tint,
                height: size(),
                width: size(),
            }}
            {...style.attrs(styles.favicon)}
        />
    );
}

/** The glyph styles. */
const styles = style.create({
    glyph: {
        flexShrink: 0,
    },
    favicon: {
        display: "inline-block",
        flexShrink: 0,
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
    },
});
