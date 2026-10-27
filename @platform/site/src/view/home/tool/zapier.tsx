import * as style from "@destack/style";

import { welcome } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { zapier } from "./zapier.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    back: ["m15 18-6-6 6-6"],
    pencil: ["M4 20h4L19 9l-4-4L4 16z", "m13.5 6.5 4 4"],
    play: ["M7 5v14l11-7z"],
    plus: ["M12 5v14", "M5 12h14"],
    minus: ["M5 12h14"],
    more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
    check: ["m6 12.5 4 4 8-9"],
    caret: ["m7 10 5 5 5-5"],
    bolt: ["M13 3 5 14h6l-1 7 8-11h-6z"],
    hook: [
        "M8.5 17a3 3 0 1 1 2.5-4.6L14 7",
        "M14 7a3 3 0 1 1 2.8 2",
        "M16.8 9h0l2.2 4.5a3 3 0 1 1-2.5 4.5H11",
    ],
} satisfies Record<string, readonly string[]>;

/** The mark of each step's app: its icon and colour, or a glyph of the editor's own. */
const marks: readonly (readonly [icon: string, tint: string])[] = [
    ["", ""],
    ["slack", "#4a154b"],
    ["gmail", "#ea4335"],
];

/** Draw the welcome automation as a zap in its editor, its Slack step chosen. */
export function Zapier() {
    return (
        <div data-component="ZapierPage" {...style.attrs(styles.page)}>
            {/* header */}
            <header {...style.attrs(styles.head)}>
                <Mark name="back" size={18} />
                <Favicon icon="zapier" tint="#ff4f00" size={20} />
                <b {...style.attrs(styles.zapName)}>Welcome new signups</b>
                <Mark name="pencil" size={14} />
                <span {...style.attrs(styles.status)}>
                    <span {...style.attrs(styles.statusDot)} />
                    On
                </span>
                <span {...style.attrs(styles.headEnd)}>
                    <span {...style.attrs(styles.testRun)}>
                        <Mark name="play" size={13} />
                        Test run
                    </span>
                    <span {...style.attrs(styles.publish)}>Publish</span>
                </span>
            </header>

            <div {...style.attrs(styles.body)}>
                {/* steps on the canvas */}
                <div {...style.attrs(styles.canvas)}>
                    {welcome.map(([app, step, detail], index) => (
                        <>
                            {index > 0 ? (
                                <span {...style.attrs(styles.joint)}>
                                    <span {...style.attrs(styles.line)} />
                                    <span {...style.attrs(styles.add)}>
                                        <Mark name="plus" size={12} />
                                    </span>
                                    <span {...style.attrs(styles.line)} />
                                </span>
                            ) : undefined}
                            <article {...style.attrs(styles.card, index === 1 && styles.cardOn)}>
                                <span {...style.attrs(styles.cardHead)}>
                                    <span {...style.attrs(styles.kind)}>
                                        {index === 0 ? <Mark name="bolt" size={12} /> : undefined}
                                        {index === 0 ? "Trigger" : "Action"}
                                    </span>
                                    <Mark name="more" size={16} />
                                </span>
                                <span {...style.attrs(styles.cardBody)}>
                                    <span {...style.attrs(styles.appMark)}>
                                        {index === 0 ? (
                                            <Mark name="hook" size={18} />
                                        ) : (
                                            <Favicon
                                                icon={marks[index]?.[0] ?? ""}
                                                tint={marks[index]?.[1] ?? ""}
                                                size={18}
                                            />
                                        )}
                                    </span>
                                    <span {...style.attrs(styles.cardText)}>
                                        <b>
                                            {index + 1}. {step}
                                        </b>
                                        <span {...style.attrs(styles.cardDetail)}>
                                            {index === 0 ? "Webhooks by Zapier" : app} · {detail}
                                        </span>
                                    </span>
                                    <span {...style.attrs(styles.done)}>
                                        <Mark name="check" size={12} />
                                    </span>
                                </span>
                            </article>
                        </>
                    ))}
                    <span {...style.attrs(styles.zoom)}>
                        <Mark name="minus" size={14} />
                        100%
                        <Mark name="plus" size={14} />
                    </span>
                </div>

                {/* the chosen step's setup */}
                <aside {...style.attrs(styles.panel)}>
                    <span {...style.attrs(styles.panelHead)}>
                        <Favicon icon="slack" tint="#4a154b" size={18} />
                        <b>2. Post in #launch</b>
                    </span>
                    <span {...style.attrs(styles.tabs)}>
                        <span {...style.attrs(styles.tabOn)}>Setup</span>
                        <span>Configure</span>
                        <span>Test</span>
                    </span>
                    <span {...style.attrs(styles.field)}>
                        <span {...style.attrs(styles.fieldName)}>App</span>
                        <span {...style.attrs(styles.input)}>
                            Slack
                            <Mark name="caret" size={12} />
                        </span>
                    </span>
                    <span {...style.attrs(styles.field)}>
                        <span {...style.attrs(styles.fieldName)}>Action event</span>
                        <span {...style.attrs(styles.input)}>
                            Send Channel Message
                            <Mark name="caret" size={12} />
                        </span>
                    </span>
                    <span {...style.attrs(styles.field)}>
                        <span {...style.attrs(styles.fieldName)}>Account</span>
                        <span {...style.attrs(styles.input)}>
                            Slack Symbol #2
                            <Mark name="caret" size={12} />
                        </span>
                    </span>
                    <span {...style.attrs(styles.continue)}>Continue</span>
                </aside>
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

/** The Zapier page styles. */
const styles = style.create({
    page: {
        backgroundColor: zapier.surface,
        color: zapier.ink,
        display: "grid",
        fontFamily: zapier.face,
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

    // header
    head: {
        alignItems: "center",
        borderBottomColor: zapier.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: zapier.muted,
        display: "flex",
        gap: "10px",
        height: "52px",
        paddingInline: "14px",
    },
    zapName: {
        color: zapier.ink,
        fontSize: "15px",
        whiteSpace: "nowrap",
    },
    status: {
        alignItems: "center",
        borderColor: zapier.line,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: zapier.ink,
        display: { default: "inline-flex", [site.narrow]: "none" },
        fontSize: "12px",
        gap: "6px",
        paddingBlock: "2px",
        paddingInline: "8px",
    },
    statusDot: {
        backgroundColor: zapier.on,
        borderRadius: "50%",
        height: "7px",
        width: "7px",
    },
    headEnd: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
        marginInlineStart: "auto",
    },
    testRun: {
        alignItems: "center",
        borderColor: zapier.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: zapier.ink,
        display: { default: "inline-flex", [site.narrow]: "none" },
        fontWeight: 600,
        gap: "6px",
        paddingBlock: "6px",
        paddingInline: "12px",
    },
    publish: {
        backgroundColor: zapier.orange,
        borderRadius: "6px",
        color: zapier.onAccent,
        fontWeight: 600,
        paddingBlock: "7px",
        paddingInline: "16px",
    },
    body: {
        display: "grid",
        gridTemplateColumns: { default: "minmax(0, 1fr) 260px", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },

    // canvas
    canvas: {
        alignContent: "start",
        backgroundColor: zapier.canvas,
        backgroundImage: `radial-gradient(${zapier.dots} 1px, transparent 1px)`,
        backgroundSize: "16px 16px",
        display: "grid",
        justifyItems: "center",
        overflow: "hidden",
        paddingBlock: "28px",
        paddingInline: "16px",
        position: "relative",
    },
    card: {
        backgroundColor: zapier.surface,
        borderColor: zapier.line,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        boxShadow: zapier.lift,
        display: "grid",
        gap: "10px",
        maxWidth: "340px",
        padding: "12px",
        width: "100%",
    },
    cardOn: {
        borderColor: zapier.purple,
        boxShadow: `0 0 0 2px ${zapier.purple}`,
    },
    cardHead: {
        alignItems: "center",
        color: zapier.muted,
        display: "flex",
        justifyContent: "space-between",
    },
    kind: {
        alignItems: "center",
        backgroundColor: zapier.badge,
        borderRadius: "4px",
        color: zapier.ink,
        display: "inline-flex",
        fontSize: "11px",
        fontWeight: 600,
        gap: "4px",
        paddingBlock: "2px",
        paddingInline: "6px",
    },
    cardBody: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
    },
    appMark: {
        alignItems: "center",
        borderColor: zapier.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: zapier.orange,
        display: "inline-flex",
        flexShrink: 0,
        height: "34px",
        justifyContent: "center",
        width: "34px",
    },
    cardText: {
        display: "grid",
        gap: "2px",
        minWidth: 0,
    },
    cardDetail: {
        color: zapier.muted,
        fontSize: "12px",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    done: {
        alignItems: "center",
        backgroundColor: zapier.on,
        borderRadius: "50%",
        color: zapier.onAccent,
        display: "inline-flex",
        flexShrink: 0,
        height: "18px",
        justifyContent: "center",
        marginInlineStart: "auto",
        width: "18px",
    },
    joint: {
        alignItems: "center",
        color: zapier.muted,
        display: "flex",
        flexDirection: "column",
    },
    line: {
        backgroundColor: zapier.line,
        height: "10px",
        width: "1px",
    },
    add: {
        alignItems: "center",
        backgroundColor: zapier.surface,
        borderColor: zapier.line,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    zoom: {
        alignItems: "center",
        backgroundColor: zapier.surface,
        borderRadius: "6px",
        bottom: "12px",
        boxShadow: zapier.lift,
        color: zapier.muted,
        display: "flex",
        gap: "10px",
        insetInlineStart: "12px",
        paddingBlock: "6px",
        paddingInline: "10px",
        position: "absolute",
    },

    // panel
    panel: {
        alignContent: "start",
        borderInlineStartColor: zapier.line,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
        display: { default: "grid", [site.narrow]: "none" },
        gap: "12px",
        padding: "16px",
    },
    panelHead: {
        alignItems: "center",
        display: "flex",
        gap: "8px",
        fontSize: "14px",
    },
    tabs: {
        borderBottomColor: zapier.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: zapier.muted,
        display: "flex",
        gap: "16px",
    },
    tabOn: {
        boxShadow: `inset 0 -2px 0 ${zapier.purple}`,
        color: zapier.ink,
        fontWeight: 600,
        paddingBlockEnd: "8px",
    },
    field: {
        display: "grid",
        gap: "6px",
    },
    fieldName: {
        fontSize: "12px",
        fontWeight: 600,
    },
    input: {
        alignItems: "center",
        borderColor: zapier.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        justifyContent: "space-between",
        paddingBlock: "7px",
        paddingInline: "10px",
    },
    continue: {
        backgroundColor: zapier.purple,
        borderRadius: "6px",
        color: zapier.onAccent,
        fontWeight: 600,
        paddingBlock: "8px",
        textAlign: "center",
    },
});
