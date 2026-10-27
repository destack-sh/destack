import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, font } from "@destack/theme/tokens.stylex";
import { createSignal, For } from "@destack/view";

import { palette } from "../../palette.stylex";
import { typeScale } from "../../layout/type.stylex";
import { AppGlyph, AppHeader, AppRail, appStyles } from "../figure/app";
import { Glyph } from "../figure/glyph";
import { CHANGE_COUNT, type Lane, REQUEST, type Step, steps } from "./session";

/** The apps on your space's rail, the log open. */
const rail: readonly (readonly [icon: string, label: string])[] = [
    ["audit", "Log"],
    ["pages", "Pages"],
    ["tasks", "Tasks"],
    ["chat", "Chat"],
    ["table", "Tables"],
    ["calendar", "Calendar"],
];

/** The slips your agent finds in the session. */
const slipCount = steps.filter((step) => step.slip !== undefined).length;

/** The slips no fix can take back. */
const permanentCount = steps.filter(
    (step) => step.slip !== undefined && step.slip.fixed === undefined,
).length;

/** Draw your space's log: your agent's session, its request, its steps, and any slips. */
export function AuditLog(properties: { lane: Lane; onTouch: () => void }) {
    // open the waitlist email until the reader opens another step
    const [opened, setOpened] = createSignal<Step | undefined>(steps[1]);
    const toggle = (step: Step) => {
        properties.onTouch();
        setOpened((current) => (current === step ? undefined : step));
    };

    // count the slips to review: none before review, all after, the permanent ones once fixed
    const review = () =>
        properties.lane === "prepared"
            ? 0
            : properties.lane === "reviewed"
              ? slipCount
              : permanentCount;

    return (
        <div {...style.attrs(appStyles.app, styles.app)}>
            <AppRail open="audit" apps={rail} morph={undefined} />
            <article {...style.attrs(styles.page)}>
                <AppHeader title="Log">
                    <span {...style.attrs(appStyles.action, styles.wide)}>Undo selected</span>
                    <span {...style.attrs(appStyles.primary)}>Undo session</span>
                </AppHeader>

                {/* the filters, typed as the log reads them */}
                <div {...style.attrs(styles.filters)}>
                    <Glyph name="filter" size={13} />
                    <span {...style.attrs(styles.token)}>
                        <span {...style.attrs(styles.key)}>by:</span> Agent
                    </span>
                    <span {...style.attrs(styles.token)}>
                        <span {...style.attrs(styles.key)}>since:</span> Thu 17:00
                    </span>
                </div>

                {/* the sessions, newest first */}
                <div role="tree" aria-label="Log" {...style.attrs(styles.tree)}>
                    {/* your agent's fixes */}
                    {properties.lane === "fixed" ? (
                        <Line
                            text="Fixed 3 changes from launch prep, at your request"
                            badge={undefined}
                            isFlagged={false}
                            stamp="Fri 09:31"
                        />
                    ) : undefined}

                    {/* the session, its request, and its steps */}
                    <Line
                        text="Prepared launch day, at your request"
                        badge={review() === 0 ? `${CHANGE_COUNT} changes` : `${review()} to review`}
                        isFlagged={review() > 0}
                        stamp="Thu 17:20"
                    />
                    <q {...style.attrs(styles.request)}>{REQUEST}</q>
                    <div {...style.attrs(styles.branch)}>
                        <For each={steps}>
                            {(step) => (
                                <StepLine
                                    step={step}
                                    lane={properties.lane}
                                    isOpen={opened() === step}
                                    onToggle={() => toggle(step)}
                                />
                            )}
                        </For>
                    </div>
                </div>
            </article>
        </div>
    );
}

/** Draw one session of your agent: what it did, how it stands, and when. */
function Line(properties: {
    text: string;
    badge: string | undefined;
    isFlagged: boolean;
    stamp: string;
}) {
    return (
        <div role="treeitem" {...style.attrs(styles.line)}>
            <span aria-hidden="true" {...style.attrs(appStyles.avatar, styles.agent)}>
                A
            </span>
            <span {...style.attrs(styles.text)}>
                <b {...style.attrs(styles.actor)}>Agent</b> {properties.text}
            </span>
            {properties.badge === undefined ? (
                <span />
            ) : (
                <span
                    {...style.attrs(
                        styles.badge,
                        properties.isFlagged ? styles.flagged : styles.neutral,
                    )}
                >
                    {properties.badge}
                </span>
            )}
            <span {...style.attrs(styles.stamp)}>{properties.stamp}</span>
        </div>
    );
}

/** Draw one step: its app, what it did, its slip once reviewed, and its changes when open. */
function StepLine(properties: { step: Step; lane: Lane; isOpen: boolean; onToggle: () => void }) {
    // show the slip once your agent has reviewed the session, and its fix once fixed
    const slip = () => (properties.lane === "prepared" ? undefined : properties.step.slip);
    const fixed = () => (properties.lane === "fixed" ? slip()?.fixed : undefined);

    return (
        <div role="treeitem" aria-expanded={properties.isOpen ? "true" : "false"}>
            <button type="button" onClick={properties.onToggle} {...style.attrs(styles.step)}>
                <span {...style.attrs(styles.caret, properties.isOpen && styles.caretOpen)}>
                    <Glyph name="caret" size={11} />
                </span>
                <AppGlyph name={properties.step.app} />
                <span {...style.attrs(styles.text)}>{properties.step.text}</span>
                {slip() === undefined ? (
                    <span {...style.attrs(styles.quiet)}>{properties.step.count}</span>
                ) : (
                    <span
                        {...style.attrs(
                            styles.badge,
                            fixed() === undefined ? styles.flagged : styles.fixed,
                        )}
                    >
                        {fixed() ?? slip()?.text}
                    </span>
                )}
            </button>
            {properties.isOpen ? (
                <div {...style.attrs(styles.changes)}>
                    <For each={properties.step.changes}>
                        {([app, text]) => (
                            <div {...style.attrs(styles.change)}>
                                <AppGlyph name={app} />
                                <span {...style.attrs(styles.text)}>{text}</span>
                            </div>
                        )}
                    </For>
                </div>
            ) : undefined}
        </div>
    );
}

/** The log styles. */
const styles = style.create({
    app: {
        backgroundColor: color.background,
        color: color.foreground,
        overflow: "hidden",
    },
    page: {
        alignContent: "start",
        display: "grid",
        gap: "0.75rem",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        minHeight: 0,
        minWidth: 0,
        overflow: "hidden",
        paddingBlock: "1.125rem",
        paddingInline: "1.5rem",
    },
    wide: {
        display: { default: "inline-flex", "@container (max-width: 479px)": "none" },
    },
    filters: {
        alignItems: "center",
        borderColor: color.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: color.mutedForeground,
        display: "flex",
        gap: "0.375rem",
        overflow: "hidden",
        paddingBlock: "0.3125rem",
        paddingInline: "0.5rem",
    },
    token: {
        backgroundColor: color.muted,
        borderRadius: "4px",
        color: color.foreground,
        fontFamily: font.code,
        fontSize: typeScale.label,
        paddingBlock: "0.125rem",
        paddingInline: "0.375rem",
        whiteSpace: "nowrap",
    },
    key: {
        color: color.mutedForeground,
    },
    tree: {
        alignContent: "start",
        display: "grid",
        fontSize: typeScale.small,
        gap: "0.125rem",
        minHeight: 0,
        overflow: "hidden",
    },
    line: {
        alignItems: "center",
        columnGap: "0.5rem",
        display: "grid",
        gridTemplateColumns: "auto minmax(0, 1fr) auto 4.5rem",
        paddingBlock: "0.3125rem",
    },
    request: {
        color: color.mutedForeground,
        display: "-webkit-box",
        fontStyle: "italic",
        lineHeight: 1.45,
        marginInlineStart: "1.625rem",
        overflow: "hidden",
        paddingBlockEnd: "0.375rem",
        WebkitBoxOrient: "vertical",
        WebkitLineClamp: 2,
    },
    branch: {
        borderInlineStartColor: color.border,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
        display: "grid",
        gap: "0.0625rem",
        marginInlineStart: "0.5625rem",
        paddingInlineStart: "0.625rem",
    },
    step: {
        alignItems: "center",
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.muted },
        },
        borderRadius: "4px",
        borderStyle: "none",
        color: "inherit",
        cursor: "pointer",
        display: "grid",
        font: "inherit",
        gap: "0.5rem",
        gridTemplateColumns: "auto auto minmax(0, 1fr) auto",
        paddingBlock: "0.3125rem",
        paddingInline: "0.25rem",
        textAlign: "start",
        width: "100%",
    },
    caret: {
        color: color.mutedForeground,
        display: "inline-flex",
        transform: "rotate(-90deg)",
    },
    caretOpen: {
        transform: "none",
    },
    changes: {
        display: "grid",
        gap: "0.125rem",
        paddingBlockEnd: "0.375rem",
        paddingInlineStart: "2.75rem",
    },
    change: {
        alignItems: "center",
        color: color.mutedForeground,
        columnGap: "0.5rem",
        display: "grid",
        fontFamily: font.code,
        fontSize: typeScale.label,
        gridTemplateColumns: "auto minmax(0, 1fr)",
        paddingBlock: "0.125rem",
    },
    agent: {
        backgroundColor: palette.ember,
    },
    actor: {
        fontWeight: 600,
    },
    text: {
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    stamp: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: typeScale.label,
        textAlign: "end",
    },
    quiet: {
        color: color.mutedForeground,
        fontSize: typeScale.label,
        whiteSpace: "nowrap",
    },
    badge: {
        borderRadius: "999px",
        fontSize: typeScale.label,
        fontWeight: 600,
        paddingBlock: "0.0625rem",
        paddingInline: "0.5rem",
        whiteSpace: "nowrap",
    },
    flagged: {
        backgroundColor: `color-mix(in srgb, ${palette.ember} 14%, transparent)`,
        color: palette.ember,
    },
    neutral: {
        backgroundColor: color.muted,
        color: color.mutedForeground,
    },
    fixed: {
        backgroundColor: `color-mix(in srgb, ${palette.teal} 14%, transparent)`,
        color: palette.teal,
    },
});
