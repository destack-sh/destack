import { color, font, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import type { JSX } from "@destack/view";

import { palette } from "../../palette.stylex";
import { MODEL } from "../figure/agent";
import { AppRail, appStyles } from "../figure/app";
import { Glyph } from "../figure/glyph";
import { paperGlyphOf } from "../figure/planet";
import { ServiceGlyph } from "../figure/tile";

/** A mock too narrow for panels side by side, which then stack. */
const NARROW = "@container (max-width: 719px)";

/** A mock too short for the log under the other panels, which then hides. */
const SHORT = "@container (max-height: 599px)";

/** The seconds the invite's trace spans. */
const SPAN = 30;

/** The minutes the request chart spans, one point a minute from 09:30. */
const MINUTES = 30;

/** The minute of the chart Florian approved the invite at, 09:40:37. */
const APPROVED_AT = 10.6;

/** The apps of your space on the rail, Observability open. */
const rail: readonly (readonly [icon: string, label: string])[] = [
    ["telemetry", "Observability"],
    ["pages", "Pages"],
    ["tasks", "Tasks"],
    ["chat", "Chat"],
    ["table", "Tables"],
    ["mail", "Mail"],
];

/** The dashboard's numbers: label, value, change, and the shape of its line over the hour. */
const stats: readonly (readonly [label: string, value: string, note: string, line: number[]])[] = [
    ["Requests", "12.4k", "+38% on last hour", [3, 3, 4, 3, 4, 9, 8, 6, 6, 5]],
    ["Errors", "0.08%", "12 retried", [1, 0, 1, 0, 0, 4, 1, 0, 0, 0]],
    ["p95", "212 ms", "-4% on last hour", [5, 5, 4, 5, 5, 6, 5, 4, 4, 4]],
    ["Spend", "$0.41", "today", [0, 1, 1, 2, 2, 4, 5, 6, 6, 7]],
];

/** The requests a minute of each service on the chart, bottom first: icon, label, and points. */
const series: readonly (readonly [icon: string, label: string, points: number[]])[] = [
    [
        "pages",
        "Pages",
        [
            38, 42, 40, 36, 44, 41, 39, 43, 40, 42, 44, 70, 96, 104, 98, 90, 84, 78, 74, 70, 66, 64,
            60, 58, 56, 55, 52, 50, 49, 48,
        ],
    ],
    [
        "accounts",
        "Accounts",
        [
            6, 5, 7, 6, 5, 6, 7, 5, 6, 6, 140, 64, 30, 22, 18, 16, 14, 12, 12, 11, 10, 10, 9, 9, 8,
            8, 8, 7, 7, 7,
        ],
    ],
    [
        "mail",
        "Mail",
        [
            4, 3, 4, 5, 4, 3, 4, 4, 5, 4, 120, 270, 20, 8, 6, 5, 5, 4, 4, 4, 4, 3, 4, 4, 3, 4, 4, 3,
            4, 4,
        ],
    ],
];

/** Where an approval stands: waiting on you, or decided. */
type Decision = "asking" | "approved" | "declined";

/** The approvals across your space: who asks, what, where it stands, and who decides when. */
const approvals: readonly (readonly [
    icon: string,
    title: string,
    decision: Decision,
    who: string,
])[] = [
    ["agent", "Post the launch thread", "asking", `${MODEL.name} asks you`],
    ["sync", "Invite 400 from the waitlist", "approved", "Florian · 09:40"],
    ["agent", "Email all 2,240 signups", "declined", "Maya · 09:12"],
];

/** The approvals waiting on you. */
const asking = approvals.filter(([, , decision]) => decision === "asking").length;

/** The mark of each approval decision. */
const decisionGlyphs = { asking: "clock", approved: "check", declined: "close" } as const;

/** Where an issue stands. */
type IssueState = "new" | "regressed" | "resolved";

/** The issues across your space: service icon and name, title, events, and where it stands. */
const issues: readonly (readonly [
    icon: string,
    service: string,
    title: string,
    events: string,
    state: IssueState,
])[] = [
    ["pages", "Pages", "404 on /pricing", "31 events", "new"],
    ["accounts", "Accounts", "Slow signup query", "p95 2.1 s", "regressed"],
    ["mail", "Mail", "Provider throttled 12 sends", "12 retried", "resolved"],
];

/** The issues still open. */
const open = issues.filter(([, , , , state]) => state !== "resolved").length;

/** One step of the invite's trace: its service, what it did, and when, in seconds. */
type Step = {
    icon: string;
    service: string;
    name: string;
    start: number;
    end: number;
};

/** The invite's steps, service by service, as fig. 3 names them. */
const steps: readonly Step[] = [
    { icon: "sync", service: "Workflows", name: "invite-400", start: 0, end: 30 },
    { icon: "search", service: "Search", name: "find 400 waiting", start: 1, end: 4 },
    { icon: "user", service: "Florian", name: "approve", start: 6, end: 6 },
    { icon: "accounts", service: "Accounts", name: "create 400", start: 8, end: 12 },
    { icon: "storage", service: "Database", name: "mark 400 invited", start: 8, end: 12 },
    { icon: "vault", service: "Vault", name: "read mail key", start: 13, end: 13.5 },
    { icon: "mail", service: "Mail", name: "send 400, retry 12", start: 14, end: 30 },
];

/** The log across every service, newest first: time, icon, service, and line. */
const log: readonly (readonly [time: string, icon: string, service: string, text: string])[] = [
    ["09:41:06", "pages", "Pages", "GET /welcome 200, from invite link"],
    ["09:41:01", "sync", "Workflows", "completed invite-400"],
    ["09:41:01", "mail", "Mail", "batch 4 of 4 sent"],
    ["09:40:58", "mail", "Mail", "retried 12 of 12"],
    ["09:40:55", "mail", "Mail", "batch 3 of 4 sent, 12 throttled"],
    ["09:40:51", "mail", "Mail", "batch 2 of 4 sent"],
    ["09:40:48", "mail", "Mail", "batch 1 of 4 sent"],
    ["09:40:44", "vault", "Vault", "read mail key for invite-400"],
    ["09:40:43", "storage", "Database", "marked 400 invited"],
    ["09:40:43", "accounts", "Accounts", "created 400"],
    ["09:40:37", "user", "Florian", "approved invite-400"],
];

/** Draw your space's launch dashboard: numbers, requests, approvals, issues, a trace, the log. */
export function LaunchDashboard() {
    return (
        <div data-component="LaunchDashboard" {...style.attrs(appStyles.app, styles.app)}>
            <AppRail open="telemetry" apps={rail} />
            <main {...style.attrs(appStyles.main, styles.main)}>
                {/* the dashboard */}
                <div {...style.attrs(styles.head)}>
                    <span {...style.attrs(styles.crumb)}>Dashboards /</span>
                    <b {...style.attrs(styles.title)}>Launch</b>
                    <span {...style.attrs(styles.live)}>
                        <span {...style.attrs(styles.liveDot)} />
                        Live
                    </span>
                    <span {...style.attrs(styles.range)}>
                        <Glyph name="clock" size={12} />
                        Last 30 minutes
                        <Glyph name="caret" size={10} />
                    </span>
                </div>

                {/* the numbers */}
                <div {...style.attrs(styles.stats)}>
                    {stats.map(([label, value, note, line]) => (
                        <span {...style.attrs(styles.panel, styles.stat)}>
                            <span {...style.attrs(styles.panelTitle)}>{label}</span>
                            <b {...style.attrs(styles.statValue)}>{value}</b>
                            <span {...style.attrs(styles.muted, styles.statNote)}>{note}</span>
                            <Spark line={line} />
                        </span>
                    ))}
                </div>

                {/* the panels */}
                <div {...style.attrs(styles.grid)}>
                    <Panel
                        title="Requests by service"
                        aside={<Legend />}
                        xstyle={styles.chartPanel}
                    >
                        <RequestChart />
                    </Panel>
                    <Panel
                        title="Approvals"
                        aside={<span {...style.attrs(styles.count)}>{asking}</span>}
                    >
                        <span {...style.attrs(styles.list)}>
                            {approvals.map(([icon, title, decision, who]) => (
                                <span {...style.attrs(styles.item)}>
                                    <ServiceGlyph icon={icon} />
                                    <span {...style.attrs(styles.itemText)}>
                                        <span {...style.attrs(styles.itemTitle)}>{title}</span>
                                        <span {...style.attrs(styles.muted)}>{who}</span>
                                    </span>
                                    {decision === "asking" ? (
                                        <span {...style.attrs(styles.buttons)}>
                                            <span {...style.attrs(styles.button)}>Decline</span>
                                            <span {...style.attrs(styles.button, styles.approve)}>
                                                Approve
                                            </span>
                                        </span>
                                    ) : (
                                        <DecisionMark decision={decision} />
                                    )}
                                </span>
                            ))}
                        </span>
                    </Panel>
                    <Panel
                        title="invite-400"
                        aside={<span {...style.attrs(styles.muted)}>30 s · completed</span>}
                    >
                        <span {...style.attrs(styles.trace)}>
                            {steps.map((step) => (
                                <StepRow step={step} />
                            ))}
                        </span>
                    </Panel>
                    <Panel
                        title="Issues"
                        aside={<span {...style.attrs(styles.count)}>{open}</span>}
                    >
                        <span {...style.attrs(styles.list)}>
                            {issues.map(([icon, service, title, events, state]) => (
                                <span {...style.attrs(styles.item)}>
                                    <ServiceGlyph icon={icon} />
                                    <span {...style.attrs(styles.itemText)}>
                                        <span {...style.attrs(styles.itemTitle)}>{title}</span>
                                        <span {...style.attrs(styles.muted)}>
                                            {service} · {events}
                                        </span>
                                    </span>
                                    <span {...style.attrs(styles.state, stateStyles[state])}>
                                        {state}
                                    </span>
                                </span>
                            ))}
                        </span>
                    </Panel>
                    <Panel title="Logs" xstyle={styles.logPanel}>
                        <span {...style.attrs(styles.log)}>
                            {log.map(([time, icon, service, text]) => (
                                <code {...style.attrs(styles.logLine)}>
                                    <span {...style.attrs(styles.muted)}>{time}</span>
                                    <span {...style.attrs(styles.who)}>
                                        <ServiceGlyph icon={icon} />
                                        <b>{service}</b>
                                    </span>
                                    <span {...style.attrs(styles.logText)}>{text}</span>
                                </code>
                            ))}
                        </span>
                    </Panel>
                </div>
            </main>
        </div>
    );
}

/** Draw one panel of the dashboard: its title, what sits beside it, and its body. */
function Panel(properties: {
    title: string;
    aside?: JSX.Element;
    xstyle?: style.Styles;
    children: JSX.Element;
}) {
    return (
        <section {...style.attrs(styles.panel, styles.panelBody, properties.xstyle)}>
            <span {...style.attrs(styles.panelHead)}>
                <b {...style.attrs(styles.panelTitle)}>{properties.title}</b>
                {properties.aside}
            </span>
            {properties.children}
        </section>
    );
}

/** Draw the requests a minute of each service, stacked, with the approval marked on the hour. */
function RequestChart() {
    // stack each service on the ones below it, scaled to the busiest minute
    const totals = Array.from({ length: MINUTES }, (_, minute) =>
        series.reduce((sum, [, , points]) => sum + (points[minute] ?? 0), 0),
    );
    const top = Math.max(...totals);
    const xOf = (minute: number) => (minute / (MINUTES - 1)) * 100;
    const yOf = (value: number) => 100 - (value / top) * 92;

    // sum each service onto the ones below it, minute by minute
    const sums = series.map(([, , points], index) =>
        points.map((_, minute) =>
            series.slice(0, index + 1).reduce((sum, [, , below]) => sum + (below[minute] ?? 0), 0),
        ),
    );
    const pointsOf = (values: readonly number[]) =>
        values.map((value, minute) => `${xOf(minute).toFixed(2)},${yOf(value).toFixed(2)}`);

    // fill each band between its own sum and the sum below it
    const bands = series.map(([icon], index) => {
        const upper = pointsOf(sums[index] ?? []);
        const lower = pointsOf(sums[index - 1] ?? Array.from({ length: MINUTES }, () => 0));

        return [icon, [...upper, ...lower.toReversed()].join(" "), upper.join(" ")] as const;
    });

    return (
        <div {...style.attrs(styles.chart)}>
            <svg
                aria-hidden="true"
                viewBox="0 0 100 100"
                preserveAspectRatio="none"
                {...style.attrs(styles.chartSvg)}
            >
                {bands.map(([icon, area, line]) => (
                    <>
                        <polygon points={area} fill={paperGlyphOf(icon)} fill-opacity="0.22" />
                        <polyline
                            points={line}
                            fill="none"
                            stroke={paperGlyphOf(icon)}
                            stroke-width="1.5"
                            vector-effect="non-scaling-stroke"
                        />
                    </>
                ))}
            </svg>
            <span
                style={{ left: `${String(xOf(APPROVED_AT))}%` }}
                {...style.attrs(styles.annotation)}
            >
                <span {...style.attrs(styles.annotationLabel)}>Florian approved invite-400</span>
            </span>
            <span {...style.attrs(styles.axis)}>
                {["09:30", "09:40", "09:50", "10:00"].map((time) => (
                    <span>{time}</span>
                ))}
            </span>
        </div>
    );
}

/** Draw the chart's legend: each service's colour and name. */
function Legend() {
    return (
        <span {...style.attrs(styles.legend)}>
            {series.map(([icon, label]) => (
                <span {...style.attrs(styles.legendItem)}>
                    <span
                        style={{ "background-color": paperGlyphOf(icon) }}
                        {...style.attrs(styles.legendSwatch)}
                    />
                    {label}
                </span>
            ))}
        </span>
    );
}

/** Draw where an approval stands, as a mark and a word. */
function DecisionMark(properties: { decision: Decision }) {
    return (
        <span {...style.attrs(styles.state, decisionStyles[properties.decision])}>
            <Glyph name={decisionGlyphs[properties.decision]} size={11} weight={2.5} />
            {properties.decision}
        </span>
    );
}

/** Draw one step: its service and what it did, and its bar on the time axis. */
function StepRow(properties: { step: Step }) {
    // place the step on the time axis in its service's colour, an instant as a dot
    const tint = () => paperGlyphOf(properties.step.icon);
    const isInstant = () => properties.step.start === properties.step.end;
    const left = () => `${String((properties.step.start / SPAN) * 100)}%`;
    const width = () => `${String(((properties.step.end - properties.step.start) / SPAN) * 100)}%`;

    return (
        <span {...style.attrs(styles.step)}>
            <span {...style.attrs(styles.who)}>
                <ServiceGlyph icon={properties.step.icon} />
                <b>{properties.step.service}</b>
                <span {...style.attrs(styles.muted, styles.operation)}>{properties.step.name}</span>
            </span>
            <span {...style.attrs(styles.track)}>
                <span
                    style={{
                        left: left(),
                        width: isInstant() ? undefined : width(),
                        "background-color": tint(),
                    }}
                    {...style.attrs(isInstant() ? styles.instant : styles.bar)}
                />
            </span>
        </span>
    );
}

/** Draw a number's line over the hour as a small sparkline. */
function Spark(properties: { line: readonly number[] }) {
    // scale the line into the box, flat along the bottom when it never rises
    const top = Math.max(1, ...properties.line);
    const points = properties.line
        .map((value, index) => {
            const x = (index / (properties.line.length - 1)) * 100;
            const y = 22 - (value / top) * 20;

            return `${x.toFixed(1)},${y.toFixed(1)}`;
        })
        .join(" ");

    return (
        <svg
            aria-hidden="true"
            viewBox="0 0 100 24"
            preserveAspectRatio="none"
            {...style.attrs(styles.spark)}
        >
            <polyline
                points={points}
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                vector-effect="non-scaling-stroke"
            />
        </svg>
    );
}

/** The dashboard styles. */
const styles = style.create({
    app: {
        containerType: "size",
        inset: 0,
        position: "absolute",
    },
    main: {
        alignContent: "start",
        display: "grid",
        gap: "0.75rem",
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        overflow: "hidden",
        paddingBlock: { default: "1.125rem", [NARROW]: "1rem" },
        paddingInline: { default: "1.25rem", [NARROW]: "1rem" },
    },
    head: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
        whiteSpace: "nowrap",
    },
    crumb: {
        color: color.mutedForeground,
        fontSize: "0.9375rem",
    },
    title: {
        fontSize: "0.9375rem",
    },
    live: {
        alignItems: "center",
        color: palette.green,
        display: "inline-flex",
        fontSize: "0.6875rem",
        fontWeight: 600,
        gap: "0.3125rem",
        marginLeft: "0.25rem",
    },
    liveDot: {
        backgroundColor: palette.lightGreen,
        borderRadius: "50%",
        height: "0.4375rem",
        width: "0.4375rem",
    },
    range: {
        alignItems: "center",
        borderColor: color.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        color: color.foreground,
        display: { default: "inline-flex", [NARROW]: "none" },
        fontSize: "0.75rem",
        gap: "0.375rem",
        marginLeft: "auto",
        paddingBlock: "0.3125rem",
        paddingInline: "0.625rem",
    },
    stats: {
        display: "grid",
        gap: "0.75rem",
        gridTemplateColumns: {
            default: "repeat(4, minmax(0, 1fr))",
            [NARROW]: "repeat(2, minmax(0, 1fr))",
        },
    },
    panel: {
        backgroundColor: color.card,
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        minWidth: 0,
    },
    stat: {
        color: palette.ember,
        columnGap: "0.75rem",
        display: "grid",
        fontSize: "0.6875rem",
        gridTemplateColumns: { default: "minmax(0, 1fr) 5rem", [NARROW]: "minmax(0, 1fr)" },
        paddingBlock: "0.625rem",
        paddingInline: "0.875rem",
        rowGap: "0.125rem",
    },
    statValue: {
        color: color.foreground,
        fontFamily: font.code,
        fontSize: "1.125rem",
        gridColumn: 1,
    },
    statNote: {
        display: { default: "inline", [SHORT]: "none" },
    },
    spark: {
        display: { default: "block", [NARROW]: "none" },
        gridColumn: 2,
        gridRow: "1 / 4",
        height: "100%",
        width: "100%",
    },
    grid: {
        display: "grid",
        gap: "0.75rem",
        gridTemplateColumns: { default: "minmax(0, 1fr) 21rem", [NARROW]: "minmax(0, 1fr)" },
        gridTemplateRows: { default: "auto auto minmax(0, 1fr)", [NARROW]: "auto" },
        minHeight: 0,
    },
    panelBody: {
        alignContent: "start",
        display: "grid",
        gap: "0.625rem",
        paddingBlock: "0.75rem",
        paddingInline: "0.875rem",
    },
    panelHead: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
        justifyContent: "space-between",
        whiteSpace: "nowrap",
    },
    panelTitle: {
        color: color.mutedForeground,
        fontSize: "0.75rem",
        fontWeight: 600,
    },
    count: {
        backgroundColor: `color-mix(in srgb, ${palette.signal} 16%, transparent)`,
        borderRadius: "999px",
        color: palette.ember,
        fontSize: "0.6875rem",
        fontWeight: 700,
        paddingInline: "0.4375rem",
    },
    legend: {
        display: { default: "flex", [NARROW]: "none" },
        gap: "0.75rem",
    },
    legendItem: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "inline-flex",
        fontSize: "0.6875rem",
        gap: "0.3125rem",
    },
    legendSwatch: {
        borderRadius: "2px",
        height: "0.5rem",
        width: "0.5rem",
    },
    chartPanel: {
        alignContent: "stretch",
        gridTemplateRows: "auto minmax(0, 1fr)",
    },
    chart: {
        alignItems: "end",
        display: "grid",
        minHeight: "7rem",
        position: "relative",
    },
    chartSvg: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        bottom: "1.25rem",
        height: "calc(100% - 1.25rem)",
        left: 0,
        position: "absolute",
        width: "100%",
    },
    annotation: {
        borderInlineStartColor: palette.ember,
        borderInlineStartStyle: "dashed",
        borderInlineStartWidth: "1.5px",
        bottom: "1.25rem",
        position: "absolute",
        top: 0,
    },
    annotationLabel: {
        backgroundColor: color.card,
        color: palette.ember,
        fontSize: "0.6875rem",
        fontWeight: 600,
        left: "0.375rem",
        position: "absolute",
        top: 0,
        whiteSpace: "nowrap",
    },
    axis: {
        color: color.mutedForeground,
        display: "flex",
        fontFamily: font.code,
        fontSize: "0.625rem",
        justifyContent: "space-between",
    },
    list: {
        display: "grid",
    },
    item: {
        alignItems: "center",
        borderTopColor: color.border,
        borderTopStyle: { default: "solid", ":first-child": "none" },
        borderTopWidth: stroke.border,
        columnGap: "0.625rem",
        display: "grid",
        gridTemplateColumns: "auto minmax(0, 1fr) auto",
        paddingBlock: "0.3125rem",
    },
    itemText: {
        display: "grid",
        fontSize: "0.6875rem",
        minWidth: 0,
        whiteSpace: "nowrap",
    },
    itemTitle: {
        color: color.foreground,
        fontSize: "0.8125rem",
        fontWeight: 600,
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    buttons: {
        display: "flex",
        gap: "0.25rem",
    },
    button: {
        borderColor: color.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        fontSize: "0.6875rem",
        fontWeight: 600,
        paddingBlock: "0.1875rem",
        paddingInline: "0.5rem",
    },
    approve: {
        backgroundColor: palette.ember,
        borderColor: palette.ember,
        color: palette.tileGlyph,
    },
    state: {
        alignItems: "center",
        borderRadius: "999px",
        display: "inline-flex",
        fontSize: "0.6875rem",
        fontWeight: 600,
        gap: "0.25rem",
        paddingBlock: "0.0625rem",
        paddingInline: "0.5rem",
    },
    trace: {
        display: "grid",
    },
    step: {
        alignItems: "center",
        columnGap: "1rem",
        display: "grid",
        fontSize: "0.75rem",
        gridTemplateColumns: { default: "14rem minmax(0, 1fr)", [NARROW]: "12rem minmax(0, 1fr)" },
        minHeight: "1.375rem",
    },
    who: {
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
    bar: {
        borderRadius: "3px",
        height: "100%",
        minWidth: "4px",
        position: "absolute",
    },
    instant: {
        borderRadius: "50%",
        height: "0.5rem",
        marginInlineStart: "-0.25rem",
        position: "absolute",
        width: "0.5rem",
    },
    logPanel: {
        display: { default: "grid", [SHORT]: "none" },
        gridColumn: "1 / -1",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: 0,
        overflow: "hidden",
    },
    log: {
        alignContent: "start",
        display: "grid",
        minHeight: 0,
        overflow: "hidden",
    },
    logLine: {
        alignItems: "center",
        columnGap: "1rem",
        display: "grid",
        fontFamily: font.code,
        fontSize: "0.6875rem",
        gridTemplateColumns: "4rem 7rem minmax(0, 1fr)",
        minHeight: "1.5rem",
        whiteSpace: "nowrap",
    },
    logText: {
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    muted: {
        color: color.mutedForeground,
    },
});

/** The colours of each approval decision. */
const decisionStyles = style.create({
    asking: {
        backgroundColor: `color-mix(in srgb, ${palette.amber} 16%, transparent)`,
        color: palette.rust,
    },
    approved: {
        backgroundColor: `color-mix(in srgb, ${palette.green} 14%, transparent)`,
        color: palette.green,
    },
    declined: {
        backgroundColor: color.muted,
        color: color.mutedForeground,
    },
});

/** The colours of each issue state. */
const stateStyles = style.create({
    new: {
        backgroundColor: `color-mix(in srgb, ${palette.mars} 14%, transparent)`,
        color: palette.ember,
    },
    regressed: {
        backgroundColor: `color-mix(in srgb, ${palette.amber} 16%, transparent)`,
        color: palette.rust,
    },
    resolved: {
        backgroundColor: `color-mix(in srgb, ${palette.green} 14%, transparent)`,
        color: palette.green,
    },
});
