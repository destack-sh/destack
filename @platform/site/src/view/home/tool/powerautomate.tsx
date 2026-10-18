import * as style from "@destack/style";
import type { JSX } from "@destack/view";

import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { powerAutomate } from "./powerautomate.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    back: ["M19 12H5", "m11 6-6 6 6 6"],
    undo: ["M9 14 4 9l5-5", "M4 9h11a5 5 0 0 1 0 10h-3"],
    redo: ["m15 14 5-5-5-5", "M20 9H9a5 5 0 0 0 0 10h3"],
    comment: ["M4 5h16v11H9l-5 4z", "M8 9h8", "M8 12h5"],
    checker: ["M6 4v5a6 6 0 0 0 12 0V4", "M12 15v2a3 3 0 0 0 6 0v-2", "M18 15a1.5 1.5 0 1 0 0-.01"],
    save: ["M5 4h11l3 3v13H5z", "M8 4v5h7V4", "M8 14h8v6H8z"],
    flask: ["M9 3h6", "M10 3v6l-5 9a2 2 0 0 0 2 3h10a2 2 0 0 0 2-3l-5-9V3"],
    copilot: ["M8 4h5a5 5 0 0 1 0 10H9", "M16 20h-5a5 5 0 0 1 0-10h4"],
    more: ["M12 5h.01", "M12 12h.01", "M12 19h.01"],
    collapse: ["m11 17-5-5 5-5", "m18 17-5-5 5-5"],
    caret: ["m7 10 5 5 5-5"],
    plus: ["M12 7v10", "M7 12h10"],
    request: ["M4 12h10", "m10 7 5 5-5 5", "M18 5v14"],
    lightning: ["M13 3 5 14h6l-1 7 8-11h-6z"],
    link: [
        "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1",
        "M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
    ],
    check: ["m6 12.5 4 4 8-9"],
} satisfies Record<string, readonly string[]>;

/** The configuration pane's tabs, Parameters chosen. */
const tabs: readonly string[] = ["Parameters", "Settings", "Code view", "About"];

/** Draw the welcome flow in the cloud flows designer, its Slack step open for editing. */
export function PowerAutomate() {
    return (
        <div data-component="PowerAutomatePage" {...style.attrs(styles.page)}>
            {/* command bar */}
            <header {...style.attrs(styles.commands)}>
                <span {...style.attrs(styles.command)}>
                    <Mark name="back" size={16} />
                    Back
                </span>
                <span {...style.attrs(styles.name)}>Welcome new signups</span>
                <span {...style.attrs(styles.commandEnd)}>
                    <span {...style.attrs(styles.icons)}>
                        <Mark name="undo" size={17} />
                        <span {...style.attrs(styles.dim)}>
                            <Mark name="redo" size={17} />
                        </span>
                        <Mark name="comment" size={17} />
                        <Mark name="checker" size={17} />
                    </span>
                    <span {...style.attrs(styles.command)}>
                        <Mark name="save" size={16} />
                        Save
                    </span>
                    <span {...style.attrs(styles.command, styles.wide)}>
                        <Mark name="flask" size={16} />
                        Test
                    </span>
                    <span {...style.attrs(styles.command, styles.wide)}>
                        <Mark name="copilot" size={16} />
                        Copilot
                    </span>
                    <span {...style.attrs(styles.command, styles.wide)}>
                        New designer
                        <span {...style.attrs(styles.toggle)}>
                            <span {...style.attrs(styles.knob)} />
                        </span>
                    </span>
                </span>
            </header>

            <div {...style.attrs(styles.body)}>
                {/* configuration pane */}
                <aside {...style.attrs(styles.pane)}>
                    <span {...style.attrs(styles.paneHead)}>
                        <Favicon icon="slack" tint="#4a154b" size={18} />
                        <b {...style.attrs(styles.paneTitle)}>Post message (V2)</b>
                        <Mark name="more" size={16} />
                        <Mark name="collapse" size={16} />
                    </span>
                    <span {...style.attrs(styles.tabs)}>
                        {tabs.map((tab) => (
                            <span
                                {...style.attrs(styles.tab, tab === "Parameters" && styles.tabOn)}
                            >
                                {tab}
                            </span>
                        ))}
                    </span>
                    <span {...style.attrs(styles.label)}>
                        Channel Name <span {...style.attrs(styles.required)}>*</span>
                    </span>
                    <span {...style.attrs(styles.field)}>
                        launch
                        <Mark name="caret" size={14} />
                    </span>
                    <span {...style.attrs(styles.label)}>
                        Message Text <span {...style.attrs(styles.required)}>*</span>
                    </span>
                    <span {...style.attrs(styles.field, styles.area)}>
                        <span>
                            New signup:{" "}
                            <span {...style.attrs(styles.token)}>
                                <Mark name="request" size={11} />
                                email
                            </span>{" "}
                            from{" "}
                            <span {...style.attrs(styles.token)}>
                                <Mark name="request" size={11} />
                                source
                            </span>
                        </span>
                    </span>
                    <span {...style.attrs(styles.label)}>Advanced parameters</span>
                    <span {...style.attrs(styles.field, styles.quiet)}>
                        Showing 0 of 7
                        <Mark name="caret" size={14} />
                    </span>
                    <span {...style.attrs(styles.connection)}>
                        <Mark name="link" size={14} />
                        Connected to Symbol Slack.
                        <span {...style.attrs(styles.linkText)}>Change connection</span>
                    </span>
                </aside>

                {/* canvas */}
                <main {...style.attrs(styles.canvas)}>
                    <div {...style.attrs(styles.flow)}>
                        <Card
                            icon={<Mark name="request" size={16} />}
                            tile="request"
                            title="When an HTTP request is received"
                            isChosen={false}
                        />
                        <Connector />
                        <Card
                            icon={<Favicon icon="slack" tint="#4a154b" size={16} />}
                            tile="plain"
                            title="Post message (V2)"
                            isChosen
                        />
                        <Connector />
                        <Card
                            icon={<Favicon icon="gmail" tint="#ea4335" size={16} />}
                            tile="plain"
                            title="Send email (V2)"
                            isChosen={false}
                        />
                        <span {...style.attrs(styles.add)}>
                            <Mark name="plus" size={14} />
                        </span>
                    </div>
                </main>
            </div>
        </div>
    );
}

/** Draw one step of the flow as a card: its connector's icon and its action's name. */
function Card(properties: {
    icon: JSX.Element;
    tile: "request" | "plain";
    title: string;
    isChosen: boolean;
}) {
    return (
        <span {...style.attrs(styles.card, properties.isChosen && styles.cardChosen)}>
            <span
                {...style.attrs(styles.tile, properties.tile === "request" && styles.tileRequest)}
            >
                {properties.icon}
            </span>
            <span {...style.attrs(styles.cardTitle)}>{properties.title}</span>
        </span>
    );
}

/** Draw the line between two cards, with the button that adds a step there. */
function Connector() {
    return (
        <span {...style.attrs(styles.connector)}>
            <span {...style.attrs(styles.line)} />
            <span {...style.attrs(styles.add, styles.addInline)}>
                <Mark name="plus" size={14} />
            </span>
            <span {...style.attrs(styles.line, styles.arrow)} />
        </span>
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

/** The Power Automate page styles. */
const styles = style.create({
    page: {
        backgroundColor: powerAutomate.canvas,
        color: powerAutomate.ink,
        display: "grid",
        fontFamily: powerAutomate.face,
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
        strokeWidth: 1.6,
    },

    // command bar
    commands: {
        alignItems: "center",
        backgroundColor: powerAutomate.panel,
        borderBottomColor: powerAutomate.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "4px",
        height: "44px",
        paddingInline: "8px",
    },
    command: {
        alignItems: "center",
        color: powerAutomate.ink,
        display: "inline-flex",
        gap: "6px",
        paddingInline: "8px",
        whiteSpace: "nowrap",
    },
    name: {
        borderInlineStartColor: powerAutomate.line,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
        overflow: "hidden",
        paddingInline: "12px",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    commandEnd: {
        alignItems: "center",
        color: powerAutomate.blue,
        display: "flex",
        gap: "4px",
        marginInlineStart: "auto",
    },
    icons: {
        alignItems: "center",
        display: { default: "flex", [site.narrow]: "none" },
        gap: "14px",
        paddingInline: "8px",
    },
    dim: {
        display: "inline-flex",
        opacity: 0.35,
    },
    wide: {
        display: { default: "inline-flex", [site.compact]: "none" },
    },
    toggle: {
        backgroundColor: powerAutomate.blue,
        borderRadius: "999px",
        display: "inline-flex",
        height: "18px",
        justifyContent: "end",
        padding: "3px",
        width: "36px",
    },
    knob: {
        backgroundColor: powerAutomate.onAccent,
        borderRadius: "50%",
        height: "12px",
        width: "12px",
    },

    // configuration pane
    body: {
        display: "grid",
        gridTemplateColumns: { default: "280px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        minHeight: 0,
    },
    pane: {
        alignContent: "start",
        backgroundColor: powerAutomate.panel,
        borderInlineEndColor: powerAutomate.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        display: { default: "grid", [site.narrow]: "none" },
        gap: "6px",
        overflow: "hidden",
        paddingBlock: "12px",
        paddingInline: "14px",
    },
    paneHead: {
        alignItems: "center",
        color: powerAutomate.muted,
        display: "flex",
        gap: "8px",
        paddingBlockEnd: "6px",
    },
    paneTitle: {
        color: powerAutomate.ink,
        flexGrow: 1,
        fontSize: "14px",
        fontWeight: 600,
    },
    tabs: {
        borderBottomColor: powerAutomate.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: powerAutomate.muted,
        display: "flex",
        gap: "14px",
        marginBlockEnd: "6px",
    },
    tab: {
        paddingBlock: "8px",
        whiteSpace: "nowrap",
    },
    tabOn: {
        boxShadow: `inset 0 -2px 0 ${powerAutomate.blue}`,
        color: powerAutomate.ink,
        fontWeight: 600,
    },
    label: {
        fontSize: "12px",
        paddingBlockStart: "4px",
    },
    required: {
        color: powerAutomate.request,
    },
    field: {
        alignItems: "center",
        borderColor: powerAutomate.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        justifyContent: "space-between",
        minHeight: "30px",
        paddingInline: "8px",
    },
    area: {
        alignItems: "start",
        lineHeight: 2,
        minHeight: "64px",
        paddingBlock: "4px",
    },
    quiet: {
        color: powerAutomate.muted,
    },
    token: {
        alignItems: "center",
        backgroundColor: powerAutomate.token,
        borderRadius: "4px",
        color: powerAutomate.request,
        display: "inline-flex",
        gap: "3px",
        lineHeight: 1.4,
        paddingInline: "5px",
    },
    connection: {
        alignItems: "center",
        color: powerAutomate.muted,
        display: "flex",
        flexWrap: "wrap",
        fontSize: "12px",
        gap: "5px",
        paddingBlockStart: "10px",
    },
    linkText: {
        color: powerAutomate.blue,
    },

    // canvas
    canvas: {
        alignItems: "center",
        backgroundImage: `radial-gradient(${powerAutomate.dots} 1px, transparent 1px)`,
        backgroundSize: "16px 16px",
        display: "flex",
        justifyContent: "center",
        overflow: "hidden",
    },
    flow: {
        alignItems: "center",
        display: "flex",
        flexDirection: "column",
    },
    card: {
        alignItems: "center",
        backgroundColor: powerAutomate.panel,
        borderInlineStartColor: "transparent",
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "4px",
        borderRadius: "4px",
        boxShadow: powerAutomate.lift,
        display: "flex",
        gap: "10px",
        paddingBlock: "10px",
        paddingInline: "10px",
        width: "220px",
    },
    cardChosen: {
        borderInlineStartColor: powerAutomate.blue,
        boxShadow: powerAutomate.liftChosen,
    },
    tile: {
        alignItems: "center",
        borderRadius: "4px",
        display: "inline-flex",
        flexShrink: 0,
        height: "26px",
        justifyContent: "center",
        width: "26px",
    },
    tileRequest: {
        backgroundColor: powerAutomate.request,
        color: powerAutomate.onAccent,
    },
    cardTitle: {
        fontWeight: 600,
        lineHeight: 1.3,
    },
    connector: {
        alignItems: "center",
        display: "flex",
        flexDirection: "column",
    },
    line: {
        backgroundColor: powerAutomate.muted,
        height: "10px",
        width: "1px",
    },
    arrow: {
        height: "12px",
    },
    add: {
        alignItems: "center",
        backgroundColor: powerAutomate.panel,
        borderColor: powerAutomate.muted,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1px",
        color: powerAutomate.blue,
        display: "inline-flex",
        height: "22px",
        justifyContent: "center",
        marginBlockStart: "10px",
        width: "22px",
    },
    addInline: {
        marginBlockStart: 0,
    },
});
