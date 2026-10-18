import * as style from "@destack/style";

import { tasks, type TaskState } from "../verb/corner";
import { Glyph } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { linear } from "./linear.stylex";

/** The groups of the issue list in the order a tracker shows them, with their names. */
const groups: readonly (readonly [state: TaskState, name: string])[] = [
    ["started", "In Progress"],
    ["review", "In Review"],
    ["todo", "Todo"],
    ["done", "Done"],
];

/** Draw the launch tasks as an issue tracker's list, seen from your own seat. */
export function Linear() {
    return (
        <div data-component="LinearPage" {...style.attrs(styles.linear)}>
            <nav {...style.attrs(styles.linSide)}>
                <span {...style.attrs(styles.linSpace)}>
                    <span {...style.attrs(styles.linSpaceMark)}>S</span>
                    <b>Symbol</b>
                    <Glyph name="caret" size={12} />
                    <span {...style.attrs(styles.linSpaceTools)}>
                        <Glyph name="search" size={15} weight={1.75} />
                        <span {...style.attrs(styles.linCompose)}>
                            <Glyph name="compose" size={14} weight={1.75} />
                        </span>
                    </span>
                </span>
                <span {...style.attrs(styles.linItem)}>
                    <Glyph name="inbox" size={15} weight={1.75} />
                    Inbox
                </span>
                <span {...style.attrs(styles.linItem)}>
                    <Glyph name="target" size={15} weight={1.75} />
                    My issues
                </span>
                <span {...style.attrs(styles.linGroup)}>Workspace</span>
                <span {...style.attrs(styles.linItem)}>
                    <Glyph name="layers" size={15} weight={1.75} />
                    Projects
                </span>
                <span {...style.attrs(styles.linItem)}>
                    <Glyph name="sliders" size={15} weight={1.75} />
                    Views
                </span>
                <span {...style.attrs(styles.linGroup)}>Your teams</span>
                <span {...style.attrs(styles.linItem)}>
                    <span {...style.attrs(styles.linTeam)}>D</span>
                    Destack
                </span>
                <span {...style.attrs(styles.linItem, styles.linSub, styles.linOn)}>
                    <Glyph name="target" size={15} weight={1.75} />
                    Issues
                </span>
                <span {...style.attrs(styles.linItem, styles.linSub)}>
                    <Glyph name="layers" size={15} weight={1.75} />
                    Projects
                </span>
                <span {...style.attrs(styles.linInvite)}>
                    <Glyph name="plus" size={14} weight={1.75} />
                    Invite people
                </span>
            </nav>
            <div {...style.attrs(styles.linMain)}>
                <span {...style.attrs(styles.linHeader)}>
                    <span {...style.attrs(styles.linTabs)}>
                        <span {...style.attrs(styles.linTab, styles.linTabOn)}>All issues</span>
                        <span {...style.attrs(styles.linTab)}>Active</span>
                        <span {...style.attrs(styles.linTab)}>Backlog</span>
                    </span>
                    <span {...style.attrs(styles.linTools)}>
                        <Glyph name="filter" size={16} weight={1.75} />
                        <span {...style.attrs(styles.linDisplay)}>
                            <Glyph name="sliders" size={14} weight={1.75} />
                            Display
                        </span>
                    </span>
                </span>
                {groups.map(([state, name]) => (
                    <>
                        <span {...style.attrs(styles.linGroupRow)}>
                            <StateMark state={state} />
                            <b {...style.attrs(styles.linGroupName)}>{name}</b>
                            <span {...style.attrs(styles.linCount)}>
                                {tasks.filter((task) => task[2] === state).length}
                            </span>
                        </span>
                        {tasks
                            .filter((task) => task[2] === state)
                            .map(([id, title, , owner, tint, due]) => (
                                <span {...style.attrs(styles.linRow)}>
                                    <span {...style.attrs(styles.linPriority)}>
                                        <span {...style.attrs(styles.linBar, styles.linBarLow)} />
                                        <span {...style.attrs(styles.linBar, styles.linBarMid)} />
                                        <span {...style.attrs(styles.linBar)} />
                                    </span>
                                    <span {...style.attrs(styles.linId)}>{id}</span>
                                    <StateMark state={state} />
                                    <span {...style.attrs(styles.linTitle)}>{title}</span>
                                    <span {...style.attrs(styles.linLabel)}>
                                        <span {...style.attrs(styles.linLabelDot)} />
                                        Launch
                                    </span>
                                    <span {...style.attrs(styles.linDue)}>{due}</span>
                                    <span
                                        style={{ "background-color": tint }}
                                        {...style.attrs(styles.linAvatar)}
                                    >
                                        {owner}
                                    </span>
                                </span>
                            ))}
                    </>
                ))}
            </div>
        </div>
    );
}

/** Draw a tracker's state icon: an empty ring, a ring filling up, or a filled check. */
function StateMark(properties: { state: TaskState }) {
    // tint each state and fill the ring as far as it has come
    const tints: Record<TaskState, string> = {
        todo: "#b4b8c0",
        started: "#f2c94c",
        review: "#26b55e",
        done: "#5e6ad2",
    };
    const tint = () => tints[properties.state];
    const share = () => (properties.state === "started" ? 0.5 : 0.75);

    return (
        <svg aria-hidden="true" viewBox="0 0 14 14" {...style.attrs(styles.state)}>
            {properties.state === "done" ? (
                <>
                    <circle cx="7" cy="7" r="6" fill={tint()} />
                    <path
                        d="m4.5 7.2 1.7 1.7 3.3-3.6"
                        fill="none"
                        stroke="#ffffff"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    />
                </>
            ) : (
                <>
                    <circle cx="7" cy="7" r="6" fill="none" stroke={tint()} stroke-width="1.5" />
                    {properties.state === "todo" ? undefined : (
                        <circle
                            cx="7"
                            cy="7"
                            r="2.5"
                            fill="none"
                            stroke={tint()}
                            stroke-width="5"
                            stroke-dasharray={`${share() * 15.7} 15.7`}
                            transform="rotate(-90 7 7)"
                        />
                    )}
                </>
            )}
        </svg>
    );
}

/** The linear page styles. */
const styles = style.create({
    linear: {
        backgroundColor: linear.surface,
        color: linear.ink,
        display: "grid",
        fontFamily: linear.face,
        fontSize: "13px",
        gridTemplateColumns: { default: "220px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
    },
    linSide: {
        alignContent: "start",
        backgroundColor: linear.panel,
        borderRightColor: linear.rule,
        borderRightStyle: "solid",
        borderRightWidth: "1px",
        display: { default: "grid", [site.narrow]: "none" },
        gap: "1px",
        paddingBlock: "12px",
        paddingInline: "10px",
    },
    linSpace: {
        alignItems: "center",
        display: "flex",
        gap: "8px",
        marginBottom: "12px",
        paddingInline: "6px",
    },
    linSpaceMark: {
        alignItems: "center",
        backgroundColor: linear.orange,
        borderRadius: "5px",
        color: "white",
        display: "flex",
        fontSize: "11px",
        fontWeight: 700,
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    linSpaceTools: {
        alignItems: "center",
        color: linear.muted,
        display: "flex",
        gap: "10px",
        marginLeft: "auto",
    },
    linCompose: {
        alignItems: "center",
        backgroundColor: linear.surface,
        borderRadius: "50%",
        boxShadow: `0 0 0 1px ${linear.ring}`,
        display: "flex",
        height: "26px",
        justifyContent: "center",
        width: "26px",
    },
    linItem: {
        alignItems: "center",
        borderRadius: "6px",
        color: linear.text,
        display: "flex",
        fontWeight: 500,
        gap: "10px",
        height: "28px",
        paddingInline: "8px",
    },
    linSub: {
        paddingLeft: "28px",
    },
    linOn: {
        backgroundColor: linear.rule,
        color: linear.ink,
    },
    linGroup: {
        color: linear.muted,
        fontSize: "12px",
        fontWeight: 500,
        marginTop: "14px",
        paddingBlock: "4px",
        paddingInline: "8px",
    },
    linTeam: {
        alignItems: "center",
        backgroundColor: linear.peach,
        borderRadius: "4px",
        color: linear.orange,
        display: "flex",
        fontSize: "10px",
        fontWeight: 700,
        height: "16px",
        justifyContent: "center",
        width: "16px",
    },
    linInvite: {
        alignItems: "center",
        color: linear.muted,
        display: "flex",
        gap: "10px",
        marginTop: "20px",
        paddingInline: "8px",
    },
    linMain: {
        alignContent: "start",
        display: "grid",
        minWidth: 0,
    },
    linHeader: {
        alignItems: "center",
        borderBottomColor: linear.divider,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        height: "44px",
        justifyContent: "space-between",
        paddingInlineStart: "16px",
        paddingInlineEnd: "20px",
    },
    linTabs: {
        display: "flex",
        gap: "6px",
    },
    linTab: {
        borderColor: linear.border,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: linear.text,
        fontSize: "12px",
        fontWeight: 500,
        paddingBlock: "3px",
        paddingInline: "10px",
    },
    linTabOn: {
        backgroundColor: linear.line,
        color: linear.ink,
    },
    linTools: {
        alignItems: "center",
        color: linear.muted,
        display: { default: "flex", [site.narrow]: "none" },
        gap: "14px",
    },
    linDisplay: {
        alignItems: "center",
        borderColor: linear.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: linear.text,
        display: "flex",
        fontSize: "12px",
        fontWeight: 500,
        gap: "6px",
        paddingBlock: "3px",
        paddingInline: "8px",
    },
    linGroupRow: {
        alignItems: "center",
        backgroundColor: linear.hover,
        display: "flex",
        gap: "10px",
        height: "36px",
        paddingInline: "24px",
    },
    linGroupName: {
        fontWeight: 500,
    },
    linCount: {
        color: linear.muted,
    },
    linRow: {
        alignItems: "center",
        borderBottomColor: linear.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "12px",
        height: "44px",
        paddingInline: { default: "24px", [site.narrow]: "16px" },
    },
    linPriority: {
        alignItems: "end",
        display: "flex",
        gap: "1.5px",
        height: "12px",
        width: "14px",
    },
    linBar: {
        backgroundColor: linear.muted,
        borderRadius: "1px",
        height: "100%",
        width: "3px",
    },
    linBarLow: {
        height: "40%",
    },
    linBarMid: {
        height: "70%",
    },
    linId: {
        color: linear.muted,
        width: "56px",
    },
    linTitle: {
        flexGrow: 1,
        fontWeight: 500,
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    linLabel: {
        alignItems: "center",
        borderColor: linear.border,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: linear.text,
        display: { default: "flex", [site.narrow]: "none" },
        fontSize: "12px",
        gap: "6px",
        paddingBlock: "1px",
        paddingInline: "8px",
    },
    linLabelDot: {
        backgroundColor: linear.orange,
        borderRadius: "50%",
        height: "8px",
        width: "8px",
    },
    linDue: {
        color: linear.muted,
        fontSize: "12px",
        width: "48px",
    },
    linAvatar: {
        alignItems: "center",
        borderRadius: "50%",
        color: "white",
        display: "flex",
        fontSize: "10px",
        fontWeight: 600,
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    state: {
        flexShrink: 0,
        height: "14px",
        width: "14px",
    },
});
