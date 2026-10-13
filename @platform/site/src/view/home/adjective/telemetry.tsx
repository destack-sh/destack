import { color, font, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";

import { palette } from "../../palette.stylex";
import { AppRail, appStyles } from "../figure/app";
import { Glyph } from "../figure/glyph";
import { paperGlyphOf } from "../figure/planet";
import { ServiceGlyph } from "../figure/tile";

/** A mock too narrow for the trace and its selected step side by side, which then stack. */
const NARROW = "@container (max-width: 719px)";

/** The seconds the trace spans. */
const SPAN = 30;

/** The ticks along the trace's time, in seconds. */
const TICKS = [0, 5, 10, 15, 20, 25, 30];

/** The apps of your space on the rail, Telemetry open. */
const rail: readonly (readonly [icon: string, label: string])[] = [
    ["telemetry", "Telemetry"],
    ["pages", "Pages"],
    ["tasks", "Tasks"],
    ["chat", "Chat"],
    ["table", "Tables"],
    ["mail", "Mail"],
];

/** The run's views, the trace open. */
const tabs = ["Trace", "Logs", "Metrics"];

/** The run's numbers: label, value, and the shape of its line over the run. */
const metrics: readonly (readonly [label: string, value: string, line: readonly number[]])[] = [
    ["Calls", "1,217", [1, 1, 2, 9, 8, 4, 6, 7, 6, 5]],
    ["Errors", "0", [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]],
    ["p95", "1.2 s", [3, 4, 3, 5, 6, 5, 4, 5, 4, 4]],
    ["Cost", "$0.004", [0, 1, 1, 2, 3, 4, 5, 6, 7, 8]],
];

/** One step of the trace: its service, what it did, its depth, and when, in seconds. */
type Step = {
    icon: string;
    service: string;
    name: string;
    depth: number;
    start: number;
    end: number;
};

/** The invite's steps, service by service, as fig. 3 names them. */
const steps: readonly Step[] = [
    { icon: "sync", service: "Workflows", name: "invite-400", depth: 0, start: 0, end: 30 },
    { icon: "search", service: "Search", name: "find 400 waiting", depth: 1, start: 1, end: 4 },
    { icon: "user", service: "Florian", name: "approve", depth: 1, start: 6, end: 6 },
    { icon: "accounts", service: "Accounts", name: "create 400", depth: 1, start: 8, end: 12 },
    { icon: "storage", service: "Database", name: "mark 400 invited", depth: 1, start: 8, end: 12 },
    { icon: "vault", service: "Vault", name: "read mail key", depth: 1, start: 13, end: 13.5 },
    { icon: "notify", service: "Notifications", name: "send 400", depth: 1, start: 14, end: 30 },
    { icon: "mail", service: "Mail", name: "batch 1 of 4", depth: 2, start: 14, end: 17 },
    { icon: "mail", service: "Mail", name: "batch 2 of 4", depth: 2, start: 17.5, end: 20.5 },
    {
        icon: "mail",
        service: "Mail",
        name: "batch 3 of 4",
        depth: 2,
        start: 21,
        end: 24,
    },
    { icon: "mail", service: "Mail", name: "retry 12 throttled", depth: 2, start: 26, end: 27 },
    { icon: "mail", service: "Mail", name: "batch 4 of 4", depth: 2, start: 27, end: 30 },
];

/** The index of the step the app opens on: the mail, with its retries. */
const SELECTED = 6;

/** The selected step's attributes: key and value. */
const attributes: readonly (readonly [key: string, value: string])[] = [
    ["Sent", "400 of 400"],
    ["Retried", "12"],
    ["Duration", "16.1 s"],
];

/** The selected step's log: time and line. */
const lines: readonly (readonly [time: string, text: string])[] = [
    ["09:40:45", "batch 1 of 4 sent"],
    ["09:40:51", "throttled 12, retrying"],
    ["09:40:53", "retried 12 of 12"],
    ["09:40:59", "batch 4 of 4 sent"],
];

/** Draw your space's Telemetry app on the invite: its numbers, its trace, and the selected step. */
export function TelemetryRun() {
    return (
        <div data-component="TelemetryRun" {...style.attrs(appStyles.app, styles.app)}>
            <AppRail open="telemetry" apps={rail} onOpen={() => undefined} />
            <main {...style.attrs(appStyles.main, styles.main)}>
                {/* the run */}
                <div {...style.attrs(styles.head)}>
                    <b {...style.attrs(styles.title)}>invite-400</b>
                    <span {...style.attrs(styles.done)}>
                        <Glyph name="check" size={11} weight={3} />
                        Completed
                    </span>
                    <span {...style.attrs(styles.tabs)}>
                        {tabs.map((name, index) => (
                            <span {...style.attrs(styles.tab, index === 0 && styles.tabOn)}>
                                {name}
                            </span>
                        ))}
                    </span>
                </div>

                {/* the run's numbers */}
                <span {...style.attrs(styles.metrics)}>
                    {metrics.map(([label, value, line]) => (
                        <span {...style.attrs(styles.metric)}>
                            <span {...style.attrs(styles.metricLabel)}>{label}</span>
                            <b {...style.attrs(styles.metricValue)}>{value}</b>
                            <Spark line={line} />
                        </span>
                    ))}
                </span>

                {/* the trace beside the selected step */}
                <div {...style.attrs(styles.body)}>
                    <div {...style.attrs(styles.trace)}>
                        <div {...style.attrs(styles.row, styles.axis)}>
                            <span />
                            <span {...style.attrs(styles.ticks)}>
                                {TICKS.map((tick) => (
                                    <span {...style.attrs(styles.tick)}>{tick}s</span>
                                ))}
                            </span>
                        </div>
                        {steps.map((step, index) => (
                            <StepRow step={step} isSelected={index === SELECTED} />
                        ))}
                    </div>
                    <aside {...style.attrs(styles.detail)}>
                        <span {...style.attrs(styles.detailHead)}>
                            <ServiceGlyph icon="notify" />
                            <b>Notifications</b>
                            <span {...style.attrs(styles.operation)}>send 400</span>
                        </span>
                        <dl {...style.attrs(styles.attributes)}>
                            {attributes.map(([key, value]) => (
                                <span {...style.attrs(styles.attribute)}>
                                    <dt {...style.attrs(styles.key)}>{key}</dt>
                                    <dd {...style.attrs(styles.value)}>{value}</dd>
                                </span>
                            ))}
                        </dl>
                        <span {...style.attrs(styles.lines)}>
                            {lines.map(([time, text]) => (
                                <code {...style.attrs(styles.line)}>
                                    <span {...style.attrs(styles.time)}>{time}</span>
                                    {text}
                                </code>
                            ))}
                        </span>
                        <span {...style.attrs(styles.source)}>
                            <ServiceGlyph icon="source" />
                            mail/send.ts
                        </span>
                    </aside>
                </div>
            </main>
        </div>
    );
}

/** Draw one step: its service and what it did, indented by depth, and its bar on the time axis. */
function StepRow(properties: { step: Step; isSelected: boolean }) {
    // place the step on the time axis in its service's colour, an instant as a dot
    const tint = () => paperGlyphOf(properties.step.icon);
    const isInstant = () => properties.step.start === properties.step.end;
    const left = () => `${String((properties.step.start / SPAN) * 100)}%`;
    const width = () => `${String(((properties.step.end - properties.step.start) / SPAN) * 100)}%`;

    return (
        <div {...style.attrs(styles.row, properties.isSelected && styles.selected)}>
            <span {...style.attrs(styles.who, styles.depth(properties.step.depth))}>
                <ServiceGlyph icon={properties.step.icon} />
                <b {...style.attrs(styles.service)}>{properties.step.service}</b>
                <span {...style.attrs(styles.operation)}>{properties.step.name}</span>
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
        </div>
    );
}

/** Draw a number's line over the run as a small sparkline. */
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

/** The Telemetry app styles. */
const styles = style.create({
    app: {
        containerType: "inline-size",
        inset: 0,
        position: "absolute",
    },
    main: {
        display: "grid",
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        gap: "1rem",
        overflow: "hidden",
        paddingBlock: { default: "1.25rem", [NARROW]: "1rem" },
        paddingInline: { default: "1.5rem", [NARROW]: "1rem" },
    },
    head: {
        alignItems: "center",
        display: "flex",
        gap: "0.75rem",
    },
    title: {
        fontFamily: font.code,
        whiteSpace: "nowrap",
        fontSize: "1.125rem",
    },
    done: {
        alignItems: "center",
        backgroundColor: `color-mix(in srgb, ${palette.green} 14%, transparent)`,
        borderRadius: "999px",
        color: palette.green,
        display: "inline-flex",
        fontSize: "0.6875rem",
        fontWeight: 600,
        gap: "0.25rem",
        paddingBlock: "0.125rem",
        paddingInline: "0.5rem",
    },
    tabs: {
        backgroundColor: color.muted,
        display: { default: "flex", [NARROW]: "none" },
        borderRadius: "8px",
        gap: "0.125rem",
        marginLeft: "auto",
        padding: "0.1875rem",
    },
    tab: {
        borderRadius: "6px",
        color: color.mutedForeground,
        fontSize: "0.75rem",
        paddingBlock: "0.25rem",
        paddingInline: "0.625rem",
    },
    tabOn: {
        backgroundColor: color.card,
        boxShadow: `0 0 0 1px ${color.border}`,
        color: color.foreground,
        fontWeight: 600,
    },
    metrics: {
        display: "grid",
        gap: "0.75rem",
        gridTemplateColumns: {
            default: "repeat(4, minmax(0, 1fr))",
            [NARROW]: "repeat(2, minmax(0, 1fr))",
        },
    },
    metric: {
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        color: palette.ember,
        display: "grid",
        gridTemplateColumns: { default: "minmax(0, 1fr) 4.5rem", [NARROW]: "minmax(0, 1fr)" },
        paddingBlock: "0.625rem",
        paddingInline: "0.875rem",
    },
    metricLabel: {
        color: color.mutedForeground,
        fontSize: "0.75rem",
    },
    metricValue: {
        color: color.foreground,
        fontFamily: font.code,
        fontSize: "1.0625rem",
        gridColumn: 1,
    },
    spark: {
        display: { default: "block", [NARROW]: "none" },
        gridColumn: 2,
        gridRow: "1 / 3",
        height: "100%",
        width: "100%",
    },
    body: {
        alignItems: "start",
        display: "grid",
        minHeight: 0,
        gap: "1rem",
        gridTemplateColumns: { default: "minmax(0, 1fr) 16rem", [NARROW]: "minmax(0, 1fr)" },
    },
    trace: {
        alignContent: "start",
        display: "grid",
        maxHeight: "100%",
        overflow: "hidden",
    },
    row: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        columnGap: "1rem",
        display: "grid",
        fontSize: "0.8125rem",
        gridTemplateColumns: { default: "15rem minmax(0, 1fr)", [NARROW]: "10rem minmax(0, 1fr)" },
        minHeight: "2.25rem",
        paddingInline: "0.5rem",
    },
    axis: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.625rem",
        minHeight: "1.5rem",
    },
    ticks: {
        display: "flex",
        justifyContent: "space-between",
    },
    tick: {
        display: { default: "inline", [NARROW]: { default: "inline", ":nth-child(even)": "none" } },
    },
    selected: {
        backgroundColor: `color-mix(in srgb, ${palette.signal} 10%, transparent)`,
        borderRadius: "6px",
    },
    who: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
        minWidth: 0,
        whiteSpace: "nowrap",
    },
    depth: (depth: number) => ({
        paddingInlineStart: `${String(depth * 1.25)}rem`,
    }),
    service: {
        fontWeight: 600,
    },
    operation: {
        color: color.mutedForeground,
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    track: {
        height: "0.625rem",
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
        height: "0.625rem",
        marginInlineStart: "-0.3125rem",
        position: "absolute",
        width: "0.625rem",
    },
    detail: {
        alignContent: "start",
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: { default: "grid", [NARROW]: "none" },
        gap: "0.875rem",
        paddingBlock: "1rem",
        paddingInline: "1rem",
    },
    detailHead: {
        alignItems: "center",
        display: "flex",
        fontSize: "0.875rem",
        gap: "0.5rem",
    },
    attributes: {
        display: "grid",
        gap: "0.375rem",
        margin: 0,
    },
    attribute: {
        display: "flex",
        fontSize: "0.8125rem",
        justifyContent: "space-between",
    },
    key: {
        color: color.mutedForeground,
    },
    value: {
        fontWeight: 600,
        margin: 0,
    },
    lines: {
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        display: "grid",
        paddingTop: "0.75rem",
    },
    line: {
        display: "flex",
        fontFamily: font.code,
        fontSize: "0.6875rem",
        gap: "0.75rem",
        lineHeight: 1.8,
        whiteSpace: "nowrap",
    },
    time: {
        color: color.mutedForeground,
    },
    source: {
        alignItems: "center",
        color: palette.ember,
        display: "flex",
        fontFamily: font.code,
        fontSize: "0.75rem",
        gap: "0.5rem",
    },
});
