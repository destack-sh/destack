import * as style from "@destack/style";

import { board, tasks } from "../verb/corner";
import { site } from "../figure/site.stylex";
import { mural } from "./mural.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    caret: ["m7 10 5 5 5-5"],
    timer: ["M12 21a8 8 0 1 0 0-16 8 8 0 0 0 0 16z", "M12 9v4l2 2", "M10 2h4"],
    eye: [
        "M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z",
        "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
    ],
    sticky: ["M5 5h14v9l-5 5H5z", "M14 19v-5h5"],
    text: ["M5 6h14", "M12 6v13"],
    shapes: ["M8 4l4 7H4z", "M15 13a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7z", "M13 4h7v7h-7z"],
    connector: ["M5 19 19 5", "M13 5h6v6"],
    template: ["M4 4h16v6H4z", "M4 14h7v6H4z", "M15 14h5v6h-5z"],
    pen: ["M4 20h4L19 9l-4-4L4 16z"],
    plus: ["M12 5v14", "M5 12h14"],
    minus: ["M5 12h14"],
} satisfies Record<string, readonly string[]>;

/** The toolbar's tools, the sticky note chosen. */
const tools: readonly (keyof typeof glyphs)[] = [
    "sticky",
    "text",
    "shapes",
    "connector",
    "template",
    "pen",
];

/** The sticky note colour of each area, by its place on the board. */
const tints: readonly ("yellow" | "green" | "blue")[] = ["yellow", "green", "blue"];

/** Draw the launch plan as a board of areas and sticky notes. */
export function Mural() {
    return (
        <div data-component="MuralPage" {...style.attrs(styles.page)}>
            {/* top bar */}
            <header {...style.attrs(styles.top)}>
                <span {...style.attrs(styles.logo)}>M</span>
                <b {...style.attrs(styles.name)}>Destack launch plan</b>
                <Mark name="caret" size={13} />
                <span {...style.attrs(styles.topEnd)}>
                    <span {...style.attrs(styles.wide)}>
                        <Mark name="timer" size={17} />
                    </span>
                    <span {...style.attrs(styles.wide)}>
                        <Mark name="eye" size={17} />
                    </span>
                    <span {...style.attrs(styles.face, styles.faceFlorian)}>F</span>
                    <span {...style.attrs(styles.face, styles.faceAgent)}>A</span>
                    <span {...style.attrs(styles.share)}>Share</span>
                </span>
            </header>

            <div {...style.attrs(styles.canvas)}>
                {/* areas of sticky notes */}
                <div {...style.attrs(styles.areas)}>
                    {board.map(([column, notes], index) => (
                        <section {...style.attrs(styles.area)}>
                            <span {...style.attrs(styles.areaHead)}>{column}</span>
                            <div {...style.attrs(styles.notes)}>
                                {notes.map((note) => {
                                    const task = tasks.find(([id]) => id === note);

                                    return (
                                        <span
                                            {...style.attrs(
                                                styles.sticky,
                                                task === undefined
                                                    ? styles.rose
                                                    : tints[index] === "green"
                                                      ? styles.green
                                                      : tints[index] === "blue"
                                                        ? styles.blue
                                                        : styles.yellow,
                                            )}
                                        >
                                            {task === undefined ? note : `${task[0]} ${task[1]}`}
                                        </span>
                                    );
                                })}
                            </div>
                        </section>
                    ))}
                </div>

                {/* toolbar */}
                <nav {...style.attrs(styles.toolbar)}>
                    {tools.map((tool) => (
                        <span {...style.attrs(styles.tool, tool === "sticky" && styles.toolOn)}>
                            <Mark name={tool} size={20} />
                        </span>
                    ))}
                </nav>

                {/* zoom */}
                <span {...style.attrs(styles.zoom)}>
                    <Mark name="minus" size={14} />
                    86%
                    <Mark name="plus" size={14} />
                </span>
            </div>
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

/** The Mural page styles. */
const styles = style.create({
    page: {
        backgroundColor: mural.canvas,
        color: mural.ink,
        display: "grid",
        fontFamily: mural.face,
        fontSize: "13px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
    },
    mark: {
        fill: "none",
        flexShrink: 0,
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 1.8,
    },
    top: {
        alignItems: "center",
        backgroundColor: mural.panel,
        borderBottomColor: mural.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: mural.muted,
        display: "flex",
        gap: "10px",
        height: "50px",
        paddingInline: "12px",
    },
    logo: {
        alignItems: "center",
        backgroundColor: mural.pink,
        borderRadius: "8px",
        color: mural.onAccent,
        display: "inline-flex",
        fontSize: "15px",
        fontWeight: 800,
        height: "30px",
        justifyContent: "center",
        width: "30px",
    },
    name: {
        color: mural.ink,
        fontSize: "15px",
        whiteSpace: "nowrap",
    },
    topEnd: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
        marginInlineStart: "auto",
    },
    wide: {
        display: { default: "flex", [site.narrow]: "none" },
    },
    face: {
        alignItems: "center",
        borderRadius: "50%",
        color: mural.onAccent,
        display: "inline-flex",
        fontSize: "11px",
        height: "26px",
        justifyContent: "center",
        width: "26px",
    },
    faceFlorian: {
        backgroundColor: site.avatar,
    },
    faceAgent: {
        backgroundColor: site.agent,
    },
    share: {
        backgroundColor: mural.pink,
        borderRadius: "6px",
        color: mural.onAccent,
        fontWeight: 700,
        paddingBlock: "7px",
        paddingInline: "16px",
    },
    canvas: {
        overflow: "hidden",
        position: "relative",
    },
    areas: {
        display: "grid",
        gap: "20px",
        gridTemplateColumns: "repeat(3, 220px)",
        insetInlineStart: { default: "84px", [site.narrow]: "16px" },
        position: "absolute",
        top: "30px",
    },
    area: {
        alignContent: "start",
        alignSelf: "start",
        backgroundColor: mural.panel,
        borderColor: mural.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        overflow: "hidden",
    },
    areaHead: {
        backgroundColor: mural.areaHead,
        fontSize: "14px",
        fontWeight: 700,
        paddingBlock: "8px",
        paddingInline: "12px",
    },
    notes: {
        display: "grid",
        gap: "8px",
        padding: "12px",
    },
    sticky: {
        boxShadow: mural.stickyLift,
        color: mural.stickyInk,
        fontSize: "12px",
        lineHeight: 1.35,
        minHeight: "56px",
        paddingBlock: "8px",
        paddingInline: "10px",
    },
    yellow: {
        backgroundColor: mural.yellow,
    },
    green: {
        backgroundColor: mural.green,
    },
    blue: {
        backgroundColor: mural.blue,
    },
    rose: {
        backgroundColor: mural.rose,
    },
    toolbar: {
        backgroundColor: mural.panel,
        borderRadius: "10px",
        boxShadow: mural.lift,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "4px",
        insetInlineStart: "14px",
        padding: "6px",
        position: "absolute",
        top: "30px",
    },
    tool: {
        alignItems: "center",
        borderRadius: "8px",
        display: "inline-flex",
        height: "38px",
        justifyContent: "center",
        width: "38px",
    },
    toolOn: {
        backgroundColor: mural.yellow,
    },
    zoom: {
        alignItems: "center",
        backgroundColor: mural.panel,
        borderRadius: "8px",
        bottom: "12px",
        boxShadow: mural.lift,
        display: "flex",
        gap: "10px",
        insetInlineEnd: "12px",
        paddingBlock: "6px",
        paddingInline: "10px",
        position: "absolute",
    },
});
