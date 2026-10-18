import * as style from "@destack/style";

import { welcome } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { n8n } from "./n8n.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    home: ["M4 11 12 4l8 7v9h-5v-6H9v6H4z"],
    plus: ["M12 5v14", "M5 12h14"],
    flows: [
        "M6 8a2 2 0 1 0 0-4 2 2 0 0 0 0 4z",
        "M18 8a2 2 0 1 0 0-4 2 2 0 0 0 0 4z",
        "M12 20a2 2 0 1 0 0-4 2 2 0 0 0 0 4z",
        "M7 7.7 11 16.3",
        "M17 7.7 13 16.3",
    ],
    credentials: ["M15 7a4 4 0 1 1-3.5 6L5 19.5V16h3v-3h2.5", "M16 8h.01"],
    history: ["M4 12a8 8 0 1 0 2.3-5.7L4 8.5", "M4 4v4.5h4.5", "M12 8v4l3 2"],
    bolt: ["M13 3 5 14h6l-1 7 8-11h-6z"],
    hook: [
        "M8.5 17a3 3 0 1 1 2.5-4.6L14 7",
        "M14 7a3 3 0 1 1 2.8 2",
        "M16.8 9h0l2.2 4.5a3 3 0 1 1-2.5 4.5H11",
    ],
    flask: ["M9 3h6", "M10 3v6l-5 9a2 2 0 0 0 2 3h10a2 2 0 0 0 2-3l-5-9V3"],
    check: ["m6 12.5 4 4 8-9"],
    minus: ["M5 12h14"],
    fit: ["M4 9V4h5", "M20 15v5h-5", "M15 4h5v5", "M9 20H4v-5"],
} satisfies Record<string, readonly string[]>;

/** The mark of each node's app after the webhook: icon and colour. */
const marks: readonly (readonly [icon: string, tint: string])[] = [
    ["", ""],
    ["slack", "#4a154b"],
    ["gmail", "#ea4335"],
];

/** The operation each node runs, as the editor names it. */
const operations: readonly string[] = ["POST /signup", "send: message", "send: message"];

/** Draw the welcome automation as a workflow of nodes in the editor, its last run green. */
export function N8n() {
    return (
        <div data-component="N8nPage" {...style.attrs(styles.page)}>
            {/* sidebar */}
            <nav {...style.attrs(styles.side)}>
                <Favicon icon="n8n" tint="#ea4b71" size={24} />
                <Mark name="plus" size={18} />
                <Mark name="home" size={18} />
                <Mark name="flows" size={18} />
                <Mark name="credentials" size={18} />
                <Mark name="history" size={18} />
            </nav>

            <div {...style.attrs(styles.main)}>
                {/* header */}
                <header {...style.attrs(styles.head)}>
                    <span {...style.attrs(styles.crumbs)}>Personal /</span>
                    <b {...style.attrs(styles.name)}>Welcome new signups</b>
                    <span {...style.attrs(styles.tag)}>launch</span>
                    <span {...style.attrs(styles.headEnd)}>
                        <span {...style.attrs(styles.tabs)}>
                            <span {...style.attrs(styles.tabOn)}>Editor</span>
                            <span>Executions</span>
                        </span>
                        <span {...style.attrs(styles.active)}>
                            Active
                            <span {...style.attrs(styles.switch)}>
                                <span {...style.attrs(styles.knob)} />
                            </span>
                        </span>
                        <span {...style.attrs(styles.share)}>Share</span>
                    </span>
                </header>

                {/* nodes */}
                <div {...style.attrs(styles.canvas)}>
                    <div {...style.attrs(styles.nodes)}>
                        {welcome.map(([app, step], index) => (
                            <>
                                {index > 0 ? (
                                    <span {...style.attrs(styles.link)}>
                                        <span {...style.attrs(styles.items)}>1 item</span>
                                    </span>
                                ) : undefined}
                                <span {...style.attrs(styles.node)}>
                                    <span
                                        {...style.attrs(styles.box, index === 0 && styles.trigger)}
                                    >
                                        {index === 0 ? (
                                            <span {...style.attrs(styles.hook)}>
                                                <Mark name="hook" size={30} />
                                            </span>
                                        ) : (
                                            <Favicon
                                                icon={marks[index]?.[0] ?? ""}
                                                tint={marks[index]?.[1] ?? ""}
                                                size={32}
                                            />
                                        )}
                                        <span {...style.attrs(styles.ran)}>
                                            <Mark name="check" size={11} />
                                        </span>
                                        {index === 0 ? (
                                            <span {...style.attrs(styles.bolt)}>
                                                <Mark name="bolt" size={12} />
                                            </span>
                                        ) : undefined}
                                    </span>
                                    <b {...style.attrs(styles.nodeName)}>
                                        {index === 0 ? "Webhook" : step}
                                    </b>
                                    <span {...style.attrs(styles.operation)}>
                                        {index === 0
                                            ? operations[index]
                                            : `${app} · ${operations[index]}`}
                                    </span>
                                </span>
                            </>
                        ))}
                    </div>
                    <span {...style.attrs(styles.zoom)}>
                        <Mark name="fit" size={15} />
                        <Mark name="plus" size={15} />
                        <Mark name="minus" size={15} />
                    </span>
                    <span {...style.attrs(styles.test)}>
                        <Mark name="flask" size={15} />
                        Test workflow
                    </span>
                </div>
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

/** The n8n page styles. */
const styles = style.create({
    page: {
        backgroundColor: n8n.canvas,
        color: n8n.ink,
        display: "grid",
        fontFamily: n8n.face,
        fontSize: "13px",
        gridTemplateColumns: { default: "56px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
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
    side: {
        alignItems: "center",
        backgroundColor: n8n.panel,
        borderInlineEndColor: n8n.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        color: n8n.muted,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "20px",
        paddingBlock: "14px",
    },
    main: {
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minWidth: 0,
    },
    head: {
        alignItems: "center",
        backgroundColor: n8n.panel,
        borderBottomColor: n8n.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "8px",
        height: "52px",
        paddingInline: "16px",
    },
    crumbs: {
        color: n8n.muted,
        display: { default: "inline", [site.compact]: "none" },
    },
    name: {
        fontSize: "15px",
        whiteSpace: "nowrap",
    },
    tag: {
        backgroundColor: n8n.canvas,
        borderColor: n8n.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: n8n.muted,
        display: { default: "inline", [site.narrow]: "none" },
        fontSize: "11px",
        paddingInline: "6px",
    },
    headEnd: {
        alignItems: "center",
        display: "flex",
        gap: "14px",
        marginInlineStart: "auto",
    },
    tabs: {
        backgroundColor: n8n.canvas,
        borderRadius: "6px",
        color: n8n.muted,
        display: { default: "flex", [site.compact]: "none" },
        gap: "2px",
        padding: "2px",
    },
    tabOn: {
        backgroundColor: n8n.panel,
        borderRadius: "4px",
        color: n8n.ink,
        fontWeight: 600,
        paddingInline: "10px",
    },
    active: {
        alignItems: "center",
        color: n8n.green,
        display: "flex",
        fontWeight: 600,
        gap: "8px",
    },
    switch: {
        alignItems: "center",
        backgroundColor: n8n.green,
        borderRadius: "999px",
        display: "inline-flex",
        height: "18px",
        justifyContent: "flex-end",
        paddingInline: "2px",
        width: "32px",
    },
    knob: {
        backgroundColor: n8n.onAccent,
        borderRadius: "50%",
        height: "14px",
        width: "14px",
    },
    share: {
        borderColor: n8n.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        paddingBlock: "5px",
        paddingInline: "12px",
    },
    canvas: {
        alignItems: "center",
        backgroundImage: `radial-gradient(${n8n.dots} 1px, transparent 1px)`,
        backgroundSize: "20px 20px",
        display: "flex",
        justifyContent: "center",
        overflow: "hidden",
        position: "relative",
    },
    nodes: {
        alignItems: "start",
        display: "flex",
        zoom: { default: 1, [site.narrow]: 0.66 },
    },
    node: {
        alignItems: "center",
        display: "flex",
        flexDirection: "column",
        gap: "3px",
        textAlign: "center",
        width: "130px",
    },
    box: {
        alignItems: "center",
        backgroundColor: n8n.panel,
        borderColor: n8n.green,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: "2px",
        boxShadow: n8n.lift,
        display: "flex",
        height: "80px",
        justifyContent: "center",
        marginBlockEnd: "8px",
        position: "relative",
        width: "80px",
    },
    trigger: {
        borderEndStartRadius: "40px",
        borderStartStartRadius: "40px",
    },
    hook: {
        color: n8n.webhook,
        display: "flex",
    },
    ran: {
        alignItems: "center",
        backgroundColor: n8n.green,
        borderRadius: "50%",
        bottom: "-8px",
        color: n8n.onAccent,
        display: "inline-flex",
        height: "16px",
        insetInlineEnd: "-8px",
        justifyContent: "center",
        position: "absolute",
        width: "16px",
    },
    bolt: {
        color: n8n.coral,
        display: "flex",
        insetInlineStart: "-18px",
        position: "absolute",
        top: "34px",
    },
    nodeName: {
        fontSize: "13px",
    },
    operation: {
        color: n8n.muted,
        fontSize: "11px",
    },
    link: {
        backgroundColor: n8n.green,
        height: "2px",
        marginBlockStart: "40px",
        position: "relative",
        width: "56px",
    },
    items: {
        color: n8n.green,
        fontSize: "10px",
        insetInlineStart: "50%",
        position: "absolute",
        top: "-16px",
        transform: "translateX(-50%)",
        whiteSpace: "nowrap",
    },
    zoom: {
        bottom: "14px",
        color: n8n.muted,
        display: "flex",
        gap: "12px",
        insetInlineStart: "14px",
        position: "absolute",
    },
    test: {
        alignItems: "center",
        backgroundColor: n8n.coral,
        borderRadius: "8px",
        bottom: "14px",
        color: n8n.onAccent,
        display: "flex",
        fontWeight: 600,
        gap: "8px",
        insetInlineStart: "50%",
        paddingBlock: "9px",
        paddingInline: "16px",
        position: "absolute",
        transform: "translateX(-50%)",
    },
});
