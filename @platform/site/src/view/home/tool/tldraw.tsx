import * as style from "@destack/style";

import { board, tasks } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { tldraw } from "./tldraw.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    more: ["M12 5h.01", "M12 12h.01", "M12 19h.01"],
    share: [
        "M18 8a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
        "M6 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
        "M18 22a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
        "m8.6 13.5 6.8 4",
        "m15.4 6.5-6.8 4",
    ],
    select: ["M5 3l14 7-6 2-2 6z"],
    hand: [
        "M8 13V6a1.5 1.5 0 0 1 3 0v6",
        "M11 11V4.5a1.5 1.5 0 0 1 3 0V11",
        "M14 11V6a1.5 1.5 0 0 1 3 0v8a6 6 0 0 1-6 6h-1a6 6 0 0 1-5-3l-2-4a1.5 1.5 0 0 1 2.5-1.5L8 13",
    ],
    draw: ["M4 20h4L19 9l-4-4L4 16z"],
    eraser: ["M8 20h12", "m4 14 9-9 6 6-7 7H8z"],
    arrow: ["M5 19 19 5", "M11 5h8v8"],
    text: ["M5 6h14", "M12 6v13"],
    note: ["M5 4h14v10l-6 6H5z", "M13 20v-6h6"],
    image: ["M4 5h16v14H4z", "m4 16 5-5 4 4 2-2 5 5", "M15 9h.01"],
    rectangle: ["M4 6h16v12H4z"],
    up: ["m6 15 6-6 6 6"],
    undo: ["M9 14 4 9l5-5", "M4 9h10a6 6 0 0 1 0 12h-3"],
    redo: ["m15 14 5-5-5-5", "M20 9H10a6 6 0 0 0 0 12h3"],
    trash: ["M4 7h16", "M9 7V4h6v3", "M6 7l1 13h10l1-13"],
    duplicate: ["M8 8h12v12H8z", "M16 8V4H4v12h4"],
    caret: ["m9 18 6-6-6-6"],
} satisfies Record<string, readonly string[]>;

/** The toolbar's tools, the selection chosen. */
const tools: readonly (keyof typeof glyphs)[] = [
    "select",
    "hand",
    "draw",
    "eraser",
    "arrow",
    "text",
    "note",
    "image",
    "rectangle",
    "up",
];

/** The style panel's colours, as tldraw names them. */
const colours: readonly string[] = [
    "#1d1d1d",
    "#9fa8b2",
    "#e085f4",
    "#ae3ec9",
    "#4465e9",
    "#4ba1f1",
    "#f1ac4b",
    "#e16919",
    "#099268",
    "#4cb05e",
    "#f87777",
    "#e03131",
];

/** The note colour of each column of the board, by its place. */
const notes: readonly ("yellow" | "green" | "sky")[] = ["yellow", "green", "sky"];

/** Draw the launch plan on the infinite canvas: one frame per column, a note per item. */
export function Tldraw() {
    return (
        <div data-component="TldrawPage" {...style.attrs(styles.page)}>
            {/* the frames and their notes */}
            <div {...style.attrs(styles.canvas)}>
                {board.map(([column, items], index) => (
                    <section {...style.attrs(styles.frame)}>
                        <span {...style.attrs(styles.frameName)}>{column}</span>
                        {items.map((item) => {
                            const task = tasks.find(([id]) => id === item);

                            return (
                                <span
                                    {...style.attrs(
                                        styles.note,
                                        notes[index] === "green"
                                            ? styles.green
                                            : notes[index] === "sky"
                                              ? styles.sky
                                              : styles.yellow,
                                    )}
                                >
                                    {task === undefined ? item : task[1]}
                                </span>
                            );
                        })}
                    </section>
                ))}
                <svg aria-hidden="true" viewBox="0 0 60 30" {...style.attrs(styles.arrow)}>
                    <path d="M3 20c13-7 29-9 49-6" />
                    <path d="m44 7 9 7-10 6" />
                </svg>
            </div>

            {/* brand and menu */}
            <span {...style.attrs(styles.brand)}>
                <Favicon icon="tldraw" tint="#1d1d1d" size={20} />
                <b>tldraw</b>
                <Mark name="more" size={16} />
            </span>

            {/* sharing and the style panel */}
            <span {...style.attrs(styles.end)}>
                <Mark name="share" size={16} />
                <span {...style.attrs(styles.share)}>Share</span>
            </span>
            <span {...style.attrs(styles.panel, styles.styles)}>
                {colours.map((colour) => (
                    <span
                        style={{ "background-color": colour }}
                        {...style.attrs(styles.swatch, colour === "#1d1d1d" && styles.swatchOn)}
                    />
                ))}
            </span>

            {/* history and the toolbar */}
            <span {...style.attrs(styles.dock)}>
                <span {...style.attrs(styles.actions)}>
                    <Mark name="undo" size={15} />
                    <Mark name="redo" size={15} />
                    <Mark name="trash" size={15} />
                    <Mark name="duplicate" size={15} />
                    <Mark name="more" size={15} />
                </span>
                <nav {...style.attrs(styles.panel, styles.toolbar)}>
                    {tools.map((tool) => (
                        <span {...style.attrs(styles.tool, tool === "select" && styles.toolOn)}>
                            <Mark name={tool} size={18} />
                        </span>
                    ))}
                </nav>
            </span>

            {/* zoom */}
            <span {...style.attrs(styles.zoom)}>
                100%
                <Mark name="caret" size={13} />
            </span>
        </div>
    );
}

/** Draw one of the app's own glyphs in the current colour. */
function Mark(properties: { name: keyof typeof glyphs; size: number }) {
    return (
        <svg
            aria-hidden="true"
            viewBox="0 0 24 24"
            style={{ height: `${properties.size}px`, width: `${properties.size}px` }}
            {...style.attrs(styles.mark)}
        >
            {glyphs[properties.name].map((path) => (
                <path d={path} />
            ))}
        </svg>
    );
}

/** The tldraw page styles. */
const styles = style.create({
    page: {
        backgroundColor: tldraw.canvas,
        color: tldraw.ink,
        fontFamily: tldraw.face,
        fontSize: "13px",
        height: "100%",
        overflow: "hidden",
        position: "relative",
    },
    mark: {
        fill: "none",
        flexShrink: 0,
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 1.7,
    },

    // the frames and their notes
    canvas: {
        display: "grid",
        gap: "40px",
        gridTemplateColumns: "repeat(3, 168px)",
        insetInlineStart: { default: "64px", [site.narrow]: "16px" },
        position: "absolute",
        top: "84px",
    },
    frame: {
        alignContent: "start",
        borderColor: tldraw.frame,
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        gap: "12px",
        padding: "14px",
        position: "relative",
    },
    frameName: {
        color: tldraw.muted,
        fontSize: "12px",
        insetInlineStart: 0,
        position: "absolute",
        top: "-20px",
    },
    note: {
        alignItems: "center",
        boxShadow: tldraw.noteLift,
        color: tldraw.noteInk,
        display: "flex",
        fontFamily: tldraw.hand,
        fontSize: "14px",
        justifyContent: "center",
        lineHeight: 1.25,
        minHeight: "72px",
        paddingInline: "10px",
        textAlign: "center",
    },
    yellow: {
        backgroundColor: tldraw.yellow,
    },
    green: {
        backgroundColor: tldraw.green,
    },
    sky: {
        backgroundColor: tldraw.sky,
    },
    arrow: {
        fill: "none",
        height: "30px",
        insetInlineStart: "158px",
        position: "absolute",
        stroke: tldraw.ink,
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 2,
        top: "40px",
        width: "60px",
    },

    // brand, sharing and the style panel
    brand: {
        alignItems: "center",
        display: "flex",
        fontSize: "16px",
        gap: "8px",
        insetInlineStart: "12px",
        position: "absolute",
        top: "12px",
    },
    end: {
        alignItems: "center",
        color: tldraw.muted,
        display: "flex",
        gap: "14px",
        insetInlineEnd: "12px",
        position: "absolute",
        top: "10px",
    },
    share: {
        backgroundColor: tldraw.blue,
        borderRadius: "6px",
        color: tldraw.onAccent,
        fontWeight: 600,
        paddingBlock: "7px",
        paddingInline: "14px",
    },
    panel: {
        backgroundColor: tldraw.panel,
        borderRadius: "10px",
        boxShadow: tldraw.lift,
    },
    styles: {
        display: { default: "grid", [site.compact]: "none" },
        gap: "10px",
        gridTemplateColumns: "repeat(4, 16px)",
        insetInlineEnd: "8px",
        padding: "12px",
        position: "absolute",
        top: "52px",
    },
    swatch: {
        borderRadius: "50%",
        height: "16px",
        width: "16px",
    },
    swatchOn: {
        outlineColor: tldraw.frame,
        outlineOffset: "3px",
        outlineStyle: "solid",
        outlineWidth: "2px",
    },

    // history, the toolbar and zoom
    dock: {
        alignItems: "center",
        bottom: "10px",
        display: "grid",
        gap: "6px",
        insetInlineStart: "50%",
        justifyItems: "center",
        position: "absolute",
        transform: "translateX(-50%)",
    },
    actions: {
        backgroundColor: tldraw.quiet,
        borderRadius: "8px",
        color: tldraw.muted,
        display: "flex",
        gap: "14px",
        paddingBlock: "7px",
        paddingInline: "12px",
    },
    toolbar: {
        display: "flex",
        gap: "2px",
        padding: "4px",
    },
    tool: {
        alignItems: "center",
        borderRadius: "8px",
        display: { default: "inline-flex", [site.narrow]: "none" },
        height: "38px",
        justifyContent: "center",
        width: "38px",
    },
    toolOn: {
        backgroundColor: tldraw.blue,
        color: tldraw.onAccent,
        display: "inline-flex",
    },
    zoom: {
        alignItems: "center",
        backgroundColor: tldraw.quiet,
        borderStartEndRadius: "8px",
        bottom: 0,
        color: tldraw.ink,
        display: { default: "flex", [site.narrow]: "none" },
        gap: "8px",
        insetInlineStart: 0,
        paddingBlock: "10px",
        paddingInline: "14px",
        position: "absolute",
    },
});
