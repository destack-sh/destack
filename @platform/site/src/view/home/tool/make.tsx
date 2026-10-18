import * as style from "@destack/style";

import { welcome } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { make } from "./make.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    back: ["m15 18-6-6 6-6"],
    pencil: ["M4 20h4L19 9l-4-4L4 16z", "m13.5 6.5 4 4"],
    play: ["M7 5v14l11-7z"],
    clock: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 7v5l3 2"],
    scenarios: [
        "M6 8a2 2 0 1 0 0-4 2 2 0 0 0 0 4z",
        "M18 8a2 2 0 1 0 0-4 2 2 0 0 0 0 4z",
        "M12 20a2 2 0 1 0 0-4 2 2 0 0 0 0 4z",
        "M7 7.7 11 16.3",
        "M17 7.7 13 16.3",
    ],
    template: ["M4 4h16v6H4z", "M4 14h7v6H4z", "M15 14h5v6h-5z"],
    connection: [
        "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1",
        "M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
    ],
    hook: [
        "M8.5 17a3 3 0 1 1 2.5-4.6L14 7",
        "M14 7a3 3 0 1 1 2.8 2",
        "M16.8 9h0l2.2 4.5a3 3 0 1 1-2.5 4.5H11",
    ],
    gear: [
        "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
        "M4 12h2",
        "M18 12h2",
        "M12 4v2",
        "M12 18v2",
        "m6.3 6.3 1.4 1.4",
        "m16.3 16.3 1.4 1.4",
        "m6.3 17.7 1.4-1.4",
        "m16.3 7.7 1.4-1.4",
    ],
    flow: ["M5 12h14", "m14 7 5 5-5 5"],
    note: ["M5 5h14v9l-5 5H5z"],
    more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
} satisfies Record<string, readonly string[]>;

/** The mark of each module's app after the webhook: icon and colour. */
const marks: readonly (readonly [icon: string, tint: string])[] = [
    ["", ""],
    ["slack", "#4a154b"],
    ["gmail", "#ea4335"],
];

/** The action of each module, as the builder names it. */
const actions: readonly string[] = ["Custom webhook", "Create a Message", "Send an Email"];

/** Draw the welcome automation as a scenario of linked modules in the builder. */
export function Make() {
    return (
        <div data-component="MakePage" {...style.attrs(styles.page)}>
            {/* side navigation */}
            <nav {...style.attrs(styles.nav)}>
                <span {...style.attrs(styles.logo)}>
                    <Favicon icon="make" tint="#ffffff" size={22} />
                </span>
                <span {...style.attrs(styles.navItem, styles.navOn)}>
                    <Mark name="scenarios" size={18} />
                </span>
                <span {...style.attrs(styles.navItem)}>
                    <Mark name="template" size={18} />
                </span>
                <span {...style.attrs(styles.navItem)}>
                    <Mark name="connection" size={18} />
                </span>
            </nav>

            <div {...style.attrs(styles.main)}>
                {/* header */}
                <header {...style.attrs(styles.head)}>
                    <Mark name="back" size={18} />
                    <b {...style.attrs(styles.name)}>Welcome new signups</b>
                    <Mark name="pencil" size={14} />
                    <span {...style.attrs(styles.headEnd)}>
                        <span {...style.attrs(styles.face)}>F</span>
                    </span>
                </header>

                {/* modules */}
                <div {...style.attrs(styles.canvas)}>
                    <div {...style.attrs(styles.modules)}>
                        {welcome.map(([app], index) => (
                            <>
                                {index > 0 ? <span {...style.attrs(styles.link)} /> : undefined}
                                <span {...style.attrs(styles.module)}>
                                    <span
                                        {...style.attrs(
                                            styles.bubble,
                                            index === 0 && styles.webhook,
                                        )}
                                    >
                                        {index === 0 ? (
                                            <Mark name="hook" size={34} />
                                        ) : (
                                            <Favicon
                                                icon={marks[index]?.[0] ?? ""}
                                                tint={marks[index]?.[1] ?? ""}
                                                size={34}
                                            />
                                        )}
                                        <span {...style.attrs(styles.number)}>{index + 1}</span>
                                    </span>
                                    <b>{index === 0 ? "Webhooks" : app}</b>
                                    <span {...style.attrs(styles.action)}>{actions[index]}</span>
                                </span>
                            </>
                        ))}
                    </div>
                </div>

                {/* bottom bar */}
                <footer {...style.attrs(styles.bar)}>
                    <span {...style.attrs(styles.run)}>
                        <Mark name="play" size={18} />
                    </span>
                    <span {...style.attrs(styles.runLabel)}>Run once</span>
                    <span {...style.attrs(styles.rule)} />
                    <span {...style.attrs(styles.schedule)}>
                        <span {...style.attrs(styles.switch)}>
                            <span {...style.attrs(styles.knob)} />
                        </span>
                        <span {...style.attrs(styles.scheduleText)}>
                            <b>Immediately</b>
                            <span>as data arrives</span>
                        </span>
                    </span>
                    <span {...style.attrs(styles.barTools)}>
                        <Mark name="gear" size={18} />
                        <Mark name="flow" size={18} />
                        <Mark name="note" size={18} />
                        <Mark name="more" size={18} />
                    </span>
                </footer>
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

/** The Make page styles. */
const styles = style.create({
    page: {
        backgroundColor: make.canvas,
        color: make.ink,
        display: "grid",
        fontFamily: make.face,
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
    nav: {
        alignItems: "center",
        backgroundColor: make.nav,
        color: make.navInk,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "8px",
        paddingBlock: "12px",
    },
    logo: {
        display: "flex",
        paddingBlockEnd: "10px",
    },
    navItem: {
        alignItems: "center",
        borderRadius: "8px",
        display: "inline-flex",
        height: "36px",
        justifyContent: "center",
        width: "36px",
    },
    navOn: {
        backgroundColor: make.purple,
        color: make.onAccent,
    },
    main: {
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        minWidth: 0,
    },
    head: {
        alignItems: "center",
        backgroundColor: make.panel,
        borderBottomColor: make.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: make.muted,
        display: "flex",
        gap: "10px",
        height: "52px",
        paddingInline: "16px",
    },
    name: {
        color: make.ink,
        fontSize: "16px",
    },
    headEnd: {
        display: "flex",
        marginInlineStart: "auto",
    },
    face: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: make.onAccent,
        display: "inline-flex",
        fontSize: "11px",
        height: "28px",
        justifyContent: "center",
        width: "28px",
    },
    canvas: {
        alignItems: "center",
        backgroundImage: `radial-gradient(${make.dots} 1px, transparent 1px)`,
        backgroundSize: "20px 20px",
        display: "flex",
        justifyContent: "center",
        overflow: "hidden",
    },
    modules: {
        alignItems: "start",
        display: "flex",
        zoom: { default: 1, [site.narrow]: 0.7 },
    },
    module: {
        alignItems: "center",
        display: "flex",
        flexDirection: "column",
        gap: "4px",
        textAlign: "center",
        width: "130px",
    },
    bubble: {
        alignItems: "center",
        backgroundColor: make.panel,
        borderRadius: "50%",
        boxShadow: make.lift,
        display: "flex",
        height: "88px",
        justifyContent: "center",
        marginBlockEnd: "8px",
        position: "relative",
        width: "88px",
    },
    webhook: {
        backgroundColor: make.webhook,
        color: make.onAccent,
    },
    number: {
        alignItems: "center",
        backgroundColor: make.panel,
        borderColor: make.line,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1px",
        color: make.ink,
        display: "inline-flex",
        fontSize: "11px",
        fontWeight: 600,
        height: "22px",
        insetInlineEnd: "0",
        justifyContent: "center",
        position: "absolute",
        top: "0",
        width: "22px",
    },
    action: {
        color: make.muted,
        fontSize: "12px",
    },
    link: {
        borderTopColor: make.link,
        borderTopStyle: "dotted",
        borderTopWidth: "4px",
        marginBlockStart: "42px",
        width: "48px",
    },
    bar: {
        alignItems: "center",
        backgroundColor: make.panel,
        borderTopColor: make.line,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        display: "flex",
        gap: "12px",
        height: "64px",
        paddingInline: "16px",
    },
    run: {
        alignItems: "center",
        backgroundColor: make.purple,
        borderRadius: "50%",
        color: make.onAccent,
        display: "inline-flex",
        height: "44px",
        justifyContent: "center",
        width: "44px",
    },
    runLabel: {
        fontWeight: 600,
    },
    rule: {
        backgroundColor: make.line,
        height: "32px",
        width: "1px",
    },
    schedule: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
    },
    switch: {
        alignItems: "center",
        backgroundColor: make.on,
        borderRadius: "999px",
        display: "inline-flex",
        height: "20px",
        justifyContent: "flex-end",
        paddingInline: "2px",
        width: "36px",
    },
    knob: {
        backgroundColor: make.onAccent,
        borderRadius: "50%",
        height: "16px",
        width: "16px",
    },
    scheduleText: {
        color: make.muted,
        display: { default: "grid", [site.narrow]: "none" },
        fontSize: "12px",
    },
    barTools: {
        color: make.muted,
        display: { default: "flex", [site.narrow]: "none" },
        gap: "16px",
        marginInlineStart: "auto",
    },
});
