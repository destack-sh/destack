import * as style from "@destack/style";
import { color, font, space } from "@destack/theme/tokens.stylex";
import { Badge } from "@destack/ui/badge";
import { Button } from "@destack/ui/button";
import { Tree, TreeItem } from "@destack/ui/tree";
import { createSignal, For, type JSX } from "@destack/view";

import { typeScale } from "../../layout/type.stylex";
import { AppGlyph, AppHeader, AppRail, appStyles } from "../figure/app";
import { AgentAvatar } from "../figure/agent";
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
    // open the archived newsletters until the reader opens another step
    const [opened, setOpened] = createSignal<Step | undefined>(steps[2]);
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
            <AppRail open="audit" apps={rail} />
            <article {...style.attrs(styles.page)}>
                <AppHeader title="Log">
                    <Button variant="ghost" xstyle={styles.wide}>
                        Undo selected
                    </Button>
                    <Button>Undo session</Button>
                </AppHeader>

                {/* the filters, typed as the log reads them */}
                <div {...style.attrs(styles.filters)}>
                    <Glyph name="filter" size={13} />
                    <Badge variant="soft" color="gray" xstyle={styles.token}>
                        <span {...style.attrs(styles.key)}>by:</span> Agent
                    </Badge>
                    <Badge variant="soft" color="gray" xstyle={styles.token}>
                        <span {...style.attrs(styles.key)}>since:</span> yesterday 22:00
                    </Badge>
                </div>

                {/* the sessions, newest first */}
                <Tree aria-label="Log" xstyle={styles.tree}>
                    {/* your agent's fixes */}
                    {properties.lane === "fixed" ? (
                        <TreeItem
                            value="fixes"
                            label={
                                <Session
                                    text="Kept 3 changes to your mail, dropped 1, at your request"
                                    stamp="08:41"
                                />
                            }
                        />
                    ) : undefined}

                    {/* the session, its request, and its steps */}
                    <TreeItem
                        value="session"
                        defaultExpanded
                        label={
                            <Session
                                text="Tidied your inbox in preview, at your request"
                                stamp="22:04"
                                badge={
                                    review() === 0 ? (
                                        <Badge variant="soft" color="gray">
                                            {CHANGE_COUNT} changes
                                        </Badge>
                                    ) : (
                                        <Badge variant="soft" color="red">
                                            {review()} to review
                                        </Badge>
                                    )
                                }
                                request={REQUEST}
                            />
                        }
                    >
                        <For each={steps}>
                            {(step) => (
                                <StepItem
                                    step={step}
                                    lane={properties.lane}
                                    isOpen={opened() === step}
                                    onToggle={() => toggle(step)}
                                />
                            )}
                        </For>
                    </TreeItem>
                </Tree>
            </article>
        </div>
    );
}

/** Draw a session of your agent's: what it did, at your request, how it stands, and when. */
function Session(properties: {
    text: string;
    stamp: string;
    badge?: JSX.Element;
    request?: string;
}) {
    return (
        <span {...style.attrs(styles.session)}>
            <AgentAvatar size="glyph" />
            <span {...style.attrs(styles.text)}>
                <b {...style.attrs(styles.actor)}>Agent</b> {properties.text}
            </span>
            {properties.badge ?? <span />}
            <span {...style.attrs(styles.stamp)}>{properties.stamp}</span>
            {properties.request === undefined ? undefined : (
                <q {...style.attrs(styles.request)}>{properties.request}</q>
            )}
        </span>
    );
}

/** Draw one step: its app, what it did, its slip once reviewed, and its changes when open. */
function StepItem(properties: { step: Step; lane: Lane; isOpen: boolean; onToggle: () => void }) {
    // show the slip once your agent has reviewed the session, and its fix once fixed
    const slip = () => (properties.lane === "prepared" ? undefined : properties.step.slip);
    const fixed = () => (properties.lane === "fixed" ? slip()?.fixed : undefined);

    return (
        <TreeItem
            value={properties.step.text}
            expanded={properties.isOpen}
            onExpandedChange={properties.onToggle}
            label={
                <span {...style.attrs(styles.step)}>
                    <AppGlyph name={properties.step.app} />
                    <span {...style.attrs(styles.text)}>{properties.step.text}</span>
                    {slip() === undefined ? (
                        <span {...style.attrs(styles.quiet)}>{properties.step.count}</span>
                    ) : fixed() === undefined ? (
                        <Badge variant="soft" color="red">
                            {slip()?.text}
                        </Badge>
                    ) : (
                        <Badge variant="soft" color="green">
                            {fixed()}
                        </Badge>
                    )}
                </span>
            }
        >
            <For each={properties.step.changes}>
                {([app, text]) => (
                    <TreeItem
                        value={`${properties.step.text} · ${text}`}
                        label={
                            <span {...style.attrs(styles.step)}>
                                <AppGlyph name={app} />
                                <span {...style.attrs(styles.text)}>{text}</span>
                            </span>
                        }
                    />
                )}
            </For>
        </TreeItem>
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
        fontFamily: font.code,
    },
    key: {
        color: color.mutedForeground,
    },
    tree: {
        minHeight: 0,
        overflow: "hidden",
    },
    request: {
        color: color.mutedForeground,
        display: "-webkit-box",
        fontStyle: "italic",
        gridColumn: "2 / -1",
        lineHeight: 1.45,
        overflow: "hidden",
        WebkitBoxOrient: "vertical",
        WebkitLineClamp: 2,
    },
    step: {
        alignItems: "center",
        display: "grid",
        gap: space[2],
        gridTemplateColumns: "auto minmax(0, 1fr) auto",
        width: "100%",
    },
    session: {
        alignItems: "center",
        columnGap: space[2],
        display: "grid",
        gridTemplateColumns: "auto minmax(0, 1fr) auto 4.5rem",
        rowGap: space[1],
        width: "100%",
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
});
