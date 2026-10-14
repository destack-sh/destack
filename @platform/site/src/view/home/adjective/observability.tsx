import { color, font, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";

import { palette } from "../../palette.stylex";
import { AgentAvatar, MODEL } from "../figure/agent";
import { AppRail, appStyles } from "../figure/app";
import { Glyph } from "../figure/glyph";
import { paperGlyphOf } from "../figure/planet";
import { ServiceGlyph } from "../figure/tile";

/** A view too narrow for the app's sections beside the issue, which then hide. */
const MEDIUM = "@container (max-width: 1099px)";

/** A view too narrow for the run beside the trace, which then hides. */
const SMALL = "@container (max-width: 819px)";

/** A view too short for the selected span's details under the trace, which then hide. */
const SHORT = "@container (max-height: 599px)";

/** The seconds the invite's trace spans. */
const SPAN = 30;

/** The apps of your space on the rail, Observability open. */
const rail: readonly (readonly [icon: string, label: string])[] = [
    ["telemetry", "Observability"],
    ["pages", "Pages"],
    ["tasks", "Tasks"],
    ["chat", "Chat"],
    ["table", "Tables"],
    ["mail", "Mail"],
];

/** The Observability app's sections: name and count, Issues open. */
const sections: readonly (readonly [name: string, count: string])[] = [
    ["Issues", "2"],
    ["Traces", ""],
    ["Logs", ""],
    ["Metrics", ""],
    ["Approvals", "1"],
    ["Costs", ""],
];

/** The issue's figures: caption and value. */
const figures: readonly (readonly [caption: string, value: string])[] = [
    ["Events", "12"],
    ["People", "400"],
    ["Agent", MODEL.name],
    ["First seen", "09:40:51"],
    ["Resolved", "09:40:58"],
];

/** The mails sent each second of the run, from its start at 09:40:31. */
const sends = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 34, 33, 33, 0, 34, 33, 33, 30, 28, 4, 2, 0, 12, 34,
    33, 33,
];

/** The seconds the provider throttled sends in. */
const throttled = new Set([23, 24]);

/** How a span ended: as planned, slowed by a fault, or decided by a person. */
type Outcome = "ok" | "fault" | "decision";

/** One span of the trace: depth, service, operation, start and end in seconds, and how it ended. */
type Span = readonly [
    depth: number,
    icon: string,
    service: string,
    operation: string,
    start: number,
    end: number,
    outcome: Outcome,
];

/** The invite's trace across every service, as fig. 3 names them. */
const spans: readonly Span[] = [
    [0, "sync", "Workflows", "invite-400", 0, 30, "ok"],
    [1, "search", "Search", "find 400 waiting", 1, 4, "ok"],
    [1, "user", "Florian", "approve invite-400", 6, 6, "decision"],
    [1, "accounts", "Accounts", "create 400", 8, 12, "ok"],
    [1, "storage", "Database", "mark 400 invited", 8, 12, "ok"],
    [1, "vault", "Vault", "read mail key", 13, 13.4, "ok"],
    [1, "notify", "Notifications", "send 400", 14, 30, "ok"],
    [2, "mail", "Mail", "batch 1 of 4", 14, 17, "ok"],
    [2, "mail", "Mail", "batch 2 of 4", 18, 21, "ok"],
    [2, "mail", "Mail", "batch 3 of 4 · 429 ×12", 21, 25, "fault"],
    [2, "mail", "Mail", "retry 12", 26, 27, "ok"],
    [2, "mail", "Mail", "batch 4 of 4", 27, 30, "ok"],
];

/** The agent run that started the invite: who speaks or which tool runs, its text, and its time. */
const run: readonly (readonly [kind: "you" | "agent" | "tool", text: string, detail: string])[] = [
    ["you", "Invite the next 400 from the waitlist once I approve.", "09:38"],
    ["tool", "tables.query signups", "0.2 s"],
    ["tool", "mail.draft invite", "0.1 s"],
    ["tool", "approvals.request invite-400", "approved 09:40:37"],
    [
        "agent",
        "Sent 400. The provider throttled 12, and Workflows retried them. All delivered.",
        "09:41",
    ],
];

/** The selected span's attributes: key and value. */
const attributes: readonly (readonly [key: string, value: string])[] = [
    ["status", "429 Too Many Requests"],
    ["retry after", "5 s"],
    ["attempts", "2"],
    ["delivered", "100 of 100"],
];

/** The selected span's log: time, level and line. */
const lines: readonly (readonly [time: string, level: "info" | "warn", text: string])[] = [
    ["09:40:52.104", "info", "sending batch 3 of 4, 100 mails"],
    ["09:40:55.871", "warn", "provider throttled 12 sends, retry after 5 s"],
    ["09:40:57.902", "info", "retried 12 of 12, all accepted"],
];

/** The issue's activity: what happened, and when. */
const activity: readonly (readonly [text: string, time: string])[] = [
    ["Detected in Notifications", "09:40:51"],
    ["Workflows retried 12 sends", "09:40:57"],
    ["Resolved automatically", "09:40:58"],
    ["Florian viewed", "09:52"],
];

/** Draw your Observability app on the invite's issue: what happened, its trace, its run. */
export function ObservabilityIssue() {
    return (
        <div data-component="ObservabilityIssue" {...style.attrs(appStyles.app, styles.app)}>
            <AppRail open="telemetry" apps={rail} />

            {/* sections */}
            <nav {...style.attrs(styles.nav)}>
                <b {...style.attrs(styles.navTitle)}>Observability</b>
                {sections.map(([name, count], index) => (
                    <span {...style.attrs(styles.navItem, index === 0 && styles.navOn)}>
                        {name}
                        {count === "" ? undefined : (
                            <span {...style.attrs(styles.navCount)}>{count}</span>
                        )}
                    </span>
                ))}
                <span {...style.attrs(styles.navLabel)}>Views</span>
                {["Launch", "Agents", "Mail"].map((name) => (
                    <span {...style.attrs(styles.navItem)}>{name}</span>
                ))}
            </nav>

            {/* issue */}
            <main {...style.attrs(styles.main)}>
                <span {...style.attrs(styles.crumb)}>Issues / OBS-41</span>
                <span {...style.attrs(styles.titleRow)}>
                    <span {...style.attrs(styles.chip, styles.resolved)}>
                        <Glyph name="check" size={10} weight={3} />
                        Resolved
                    </span>
                    <b {...style.attrs(styles.title)}>Mail provider throttled invite-400</b>
                    <span {...style.attrs(styles.actions)}>
                        <span {...style.attrs(styles.button)}>Reopen</span>
                        <span {...style.attrs(styles.button)}>Share</span>
                    </span>
                </span>
                <span {...style.attrs(styles.summary)}>
                    Batch 3 hit the provider's rate limit after 230 sends. Workflows waited out the
                    limit and retried the 12, and every invite was delivered.
                </span>

                {/* figures and the sends over the run */}
                <span {...style.attrs(styles.figures)}>
                    {figures.map(([caption, value]) => (
                        <span {...style.attrs(styles.figure)}>
                            <span {...style.attrs(styles.caption)}>{caption}</span>
                            <b {...style.attrs(styles.value)}>{value}</b>
                        </span>
                    ))}
                    <span {...style.attrs(styles.figure, styles.histogram)}>
                        <span {...style.attrs(styles.caption)}>Sends a second</span>
                        <span {...style.attrs(styles.bars)}>
                            {sends.map((count, second) => (
                                <span
                                    style={{
                                        height: `${String(Math.max(2, (count / 34) * 100))}%`,
                                    }}
                                    {...style.attrs(
                                        styles.bar,
                                        throttled.has(second) && styles.barFault,
                                    )}
                                />
                            ))}
                        </span>
                    </span>
                </span>

                {/* trace */}
                <span {...style.attrs(styles.tabs)}>
                    {["Trace", "Logs", "Events", "Details"].map((tab, index) => (
                        <span {...style.attrs(styles.tab, index === 0 && styles.tabOn)}>{tab}</span>
                    ))}
                </span>
                <div {...style.attrs(styles.trace)}>
                    {spans.map((span) => (
                        <SpanRow span={span} />
                    ))}
                </div>

                {/* the selected span */}
                <section {...style.attrs(styles.detail)}>
                    <span {...style.attrs(styles.detailHead)}>
                        <ServiceGlyph icon="mail" />
                        <b>Mail</b>
                        <span {...style.attrs(styles.muted)}>batch 3 of 4 · 4.0 s</span>
                    </span>
                    <span {...style.attrs(styles.attributes)}>
                        {attributes.map(([key, value]) => (
                            <span {...style.attrs(styles.attribute)}>
                                <span {...style.attrs(styles.caption)}>{key}</span>
                                <code {...style.attrs(styles.attributeValue)}>{value}</code>
                            </span>
                        ))}
                    </span>
                    <span {...style.attrs(styles.log)}>
                        {lines.map(([time, level, text]) => (
                            <code {...style.attrs(styles.logLine, level === "warn" && styles.warn)}>
                                <span {...style.attrs(styles.muted)}>{time}</span>
                                <span>{level}</span>
                                <span>{text}</span>
                            </code>
                        ))}
                    </span>
                </section>
            </main>

            {/* the run behind it */}
            <aside {...style.attrs(styles.side)}>
                <span {...style.attrs(styles.sideHead)}>
                    <AgentAvatar size="glyph" />
                    <b>Started by {MODEL.name}</b>
                    <span {...style.attrs(styles.muted)}>run-7f2c</span>
                </span>
                {run.map(([kind, text, detail]) =>
                    kind === "tool" ? (
                        <span {...style.attrs(styles.tool)}>
                            <Glyph name="sliders" size={11} />
                            <code {...style.attrs(styles.toolName)}>{text}</code>
                            <span {...style.attrs(styles.muted)}>{detail}</span>
                        </span>
                    ) : (
                        <span {...style.attrs(styles.message, kind === "you" && styles.messageYou)}>
                            <span {...style.attrs(styles.messageHead)}>
                                {kind === "you" ? (
                                    <span {...style.attrs(styles.face)}>F</span>
                                ) : (
                                    <AgentAvatar size="glyph" />
                                )}
                                <b>{kind === "you" ? "Florian" : MODEL.name}</b>
                                <span {...style.attrs(styles.muted)}>{detail}</span>
                            </span>
                            {text}
                        </span>
                    ),
                )}

                {/* activity */}
                <span {...style.attrs(styles.sideLabel)}>Activity</span>
                {activity.map(([text, time]) => (
                    <span {...style.attrs(styles.event)}>
                        <span {...style.attrs(styles.eventDot)} />
                        <span>{text}</span>
                        <span {...style.attrs(styles.muted)}>{time}</span>
                    </span>
                ))}
            </aside>
        </div>
    );
}

/** Draw one span: its service and operation indented by depth, its bar, and its duration. */
function SpanRow(properties: { span: Span }) {
    // read the span, and write its duration, or the decision an instant records
    const [depth, icon, service, operation, start, end, outcome] = properties.span;
    const isInstant = start === end;
    const duration = isInstant ? "approved" : `${(end - start).toFixed(1)} s`;

    return (
        <span {...style.attrs(styles.span, outcome === "fault" && styles.spanFault)}>
            <span
                {...style.attrs(styles.spanName)}
                style={{ "padding-inline-start": `${String(depth * 1.125)}rem` }}
            >
                <ServiceGlyph icon={icon} />
                <b>{service}</b>
                <span {...style.attrs(styles.muted, styles.operation)}>{operation}</span>
            </span>
            <span {...style.attrs(styles.track)}>
                <span
                    style={{
                        left: `${String((start / SPAN) * 100)}%`,
                        width: isInstant ? undefined : `${String(((end - start) / SPAN) * 100)}%`,
                        "background-color":
                            outcome === "fault" ? palette.ember : paperGlyphOf(icon),
                    }}
                    {...style.attrs(isInstant ? styles.instant : styles.spanBar)}
                />
            </span>
            <span {...style.attrs(styles.duration, outcome === "decision" && styles.decision)}>
                {duration}
            </span>
        </span>
    );
}

/** The Observability app styles. */
const styles = style.create({
    app: {
        containerType: "size",
        gridTemplateColumns: {
            default: "3rem 12rem minmax(0, 1fr) 21rem",
            [MEDIUM]: "3rem minmax(0, 1fr) 19rem",
            [SMALL]: "3rem minmax(0, 1fr)",
        },
        gridTemplateRows: "minmax(0, 1fr)",
        inset: 0,
        position: "absolute",
    },
    nav: {
        alignContent: "start",
        borderInlineEndColor: color.border,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: stroke.border,
        display: { default: "grid", [MEDIUM]: "none" },
        gap: "0.125rem",
        paddingBlock: "1rem",
        paddingInline: "0.625rem",
    },
    navTitle: {
        fontSize: "0.9375rem",
        paddingBottom: "0.5rem",
        paddingInline: "0.5rem",
    },
    navItem: {
        alignItems: "center",
        borderRadius: "6px",
        color: color.mutedForeground,
        display: "flex",
        fontSize: "0.8125rem",
        justifyContent: "space-between",
        paddingBlock: "0.375rem",
        paddingInline: "0.5rem",
    },
    navOn: {
        backgroundColor: color.muted,
        color: color.foreground,
        fontWeight: 600,
    },
    navCount: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.6875rem",
    },
    navLabel: {
        color: color.mutedForeground,
        fontSize: "0.6875rem",
        fontWeight: 600,
        letterSpacing: "0.06em",
        paddingInline: "0.5rem",
        paddingTop: "1rem",
        textTransform: "uppercase",
    },
    main: {
        alignContent: "start",
        display: "grid",
        gap: "0.75rem",
        minWidth: 0,
        overflow: "hidden",
        paddingBlock: "1rem",
        paddingInline: "1.5rem",
    },
    crumb: {
        color: color.mutedForeground,
        fontSize: "0.75rem",
    },
    titleRow: {
        alignItems: "center",
        display: "flex",
        gap: "0.625rem",
        minWidth: 0,
    },
    title: {
        fontSize: "1.25rem",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    chip: {
        alignItems: "center",
        borderRadius: "5px",
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "0.6875rem",
        fontWeight: 600,
        gap: "0.25rem",
        paddingBlock: "0.1875rem",
        paddingInline: "0.5rem",
    },
    resolved: {
        backgroundColor: `color-mix(in srgb, ${palette.green} 14%, transparent)`,
        color: palette.green,
    },
    actions: {
        display: { default: "flex", [SMALL]: "none" },
        gap: "0.375rem",
        marginLeft: "auto",
    },
    button: {
        borderColor: color.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        fontSize: "0.75rem",
        fontWeight: 600,
        paddingBlock: "0.3125rem",
        paddingInline: "0.75rem",
    },
    summary: {
        color: color.mutedForeground,
        fontSize: "0.8125rem",
        lineHeight: 1.5,
        maxWidth: "40rem",
    },
    figures: {
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "grid",
        gridTemplateColumns: {
            default: "repeat(5, auto) minmax(8rem, 1fr)",
            [SMALL]: "repeat(3, minmax(0, 1fr))",
        },
        overflow: "hidden",
    },
    figure: {
        borderInlineEndColor: color.border,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: stroke.border,
        display: "grid",
        gap: "0.25rem",
        paddingBlock: "0.625rem",
        paddingInline: "0.875rem",
    },
    caption: {
        color: color.mutedForeground,
        fontSize: "0.6875rem",
        whiteSpace: "nowrap",
    },
    value: {
        fontFamily: font.code,
        fontSize: "0.9375rem",
        whiteSpace: "nowrap",
    },
    histogram: {
        borderInlineEndWidth: 0,
        display: { default: "grid", [SMALL]: "none" },
        gridTemplateRows: "auto minmax(0, 1fr)",
    },
    bars: {
        alignItems: "end",
        display: "flex",
        gap: "2px",
        height: "1.75rem",
    },
    bar: {
        backgroundColor: color.mutedForeground,
        borderRadius: "1px",
        flexGrow: 1,
        opacity: 0.45,
    },
    barFault: {
        backgroundColor: palette.ember,
        opacity: 1,
    },
    tabs: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "flex",
        gap: "1.25rem",
        marginTop: "0.25rem",
    },
    tab: {
        color: color.mutedForeground,
        fontSize: "0.8125rem",
        paddingBottom: "0.5rem",
    },
    tabOn: {
        boxShadow: `inset 0 -2px 0 ${palette.ember}`,
        color: color.foreground,
        fontWeight: 600,
    },
    trace: {
        alignContent: "start",
        display: "grid",
        minHeight: 0,
    },
    span: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        columnGap: "1rem",
        display: "grid",
        fontSize: "0.75rem",
        gridTemplateColumns: {
            default: "16rem minmax(0, 1fr) 4.5rem",
            [SMALL]: "12rem minmax(0, 1fr) 4rem",
        },
        minHeight: "1.625rem",
        paddingInline: "0.375rem",
    },
    spanFault: {
        backgroundColor: `color-mix(in srgb, ${palette.ember} 9%, transparent)`,
    },
    spanName: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
        minWidth: 0,
        whiteSpace: "nowrap",
    },
    operation: {
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    track: {
        height: "0.5rem",
        position: "relative",
    },
    spanBar: {
        borderRadius: "2px",
        height: "100%",
        minWidth: "3px",
        position: "absolute",
    },
    instant: {
        backgroundColor: palette.teal,
        borderRadius: "50%",
        height: "0.5rem",
        marginInlineStart: "-0.25rem",
        position: "absolute",
        width: "0.5rem",
    },
    duration: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        textAlign: "end",
    },
    decision: {
        color: palette.teal,
    },
    side: {
        alignContent: "start",
        backgroundColor: color.card,
        borderInlineStartColor: color.border,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: stroke.border,
        display: { default: "grid", [SMALL]: "none" },
        gap: "0.625rem",
        overflow: "hidden",
        paddingBlock: "1rem",
        paddingInline: "1rem",
    },
    sideHead: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "flex",
        fontSize: "0.8125rem",
        gap: "0.5rem",
        paddingBottom: "0.75rem",
    },
    message: {
        display: "grid",
        fontSize: "0.8125rem",
        gap: "0.25rem",
        lineHeight: 1.5,
    },
    messageYou: {
        backgroundColor: color.muted,
        borderRadius: "8px",
        paddingBlock: "0.5rem",
        paddingInline: "0.625rem",
    },
    messageHead: {
        alignItems: "center",
        display: "flex",
        fontSize: "0.75rem",
        gap: "0.375rem",
    },
    face: {
        alignItems: "center",
        backgroundColor: palette.teal,
        borderRadius: "50%",
        color: palette.tileGlyph,
        display: "inline-flex",
        fontSize: "0.5625rem",
        fontWeight: 700,
        height: "1rem",
        justifyContent: "center",
        width: "1rem",
    },
    tool: {
        alignItems: "center",
        borderColor: color.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        color: color.mutedForeground,
        display: "grid",
        fontSize: "0.6875rem",
        gap: "0.5rem",
        gridTemplateColumns: "auto minmax(0, 1fr) auto",
        paddingBlock: "0.375rem",
        paddingInline: "0.625rem",
    },
    toolName: {
        color: color.foreground,
        fontFamily: font.code,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    muted: {
        color: color.mutedForeground,
        fontSize: "0.6875rem",
    },
    detail: {
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: { default: "grid", [SHORT]: "none" },
        gap: "0.625rem",
        paddingBlock: "0.75rem",
        paddingInline: "0.875rem",
    },
    detailHead: {
        alignItems: "center",
        display: "flex",
        fontSize: "0.8125rem",
        gap: "0.5rem",
    },
    attributes: {
        display: "grid",
        gap: "0.75rem",
        gridTemplateColumns: "repeat(4, minmax(0, 1fr))",
    },
    attribute: {
        display: "grid",
        gap: "0.125rem",
    },
    attributeValue: {
        fontFamily: font.code,
        fontSize: "0.75rem",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    log: {
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        display: "grid",
        paddingTop: "0.5rem",
    },
    logLine: {
        columnGap: "0.875rem",
        display: "grid",
        fontFamily: font.code,
        fontSize: "0.6875rem",
        gridTemplateColumns: "6.5rem 2.5rem minmax(0, 1fr)",
        lineHeight: 1.8,
        whiteSpace: "nowrap",
    },
    warn: {
        color: palette.ember,
    },
    sideLabel: {
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        color: color.mutedForeground,
        fontSize: "0.6875rem",
        fontWeight: 600,
        letterSpacing: "0.06em",
        marginTop: "0.25rem",
        paddingTop: "0.875rem",
        textTransform: "uppercase",
    },
    event: {
        alignItems: "center",
        columnGap: "0.5rem",
        display: "grid",
        fontSize: "0.75rem",
        gridTemplateColumns: "auto minmax(0, 1fr) auto",
    },
    eventDot: {
        backgroundColor: color.mutedForeground,
        borderRadius: "50%",
        height: "0.375rem",
        width: "0.375rem",
    },
});
