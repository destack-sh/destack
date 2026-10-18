import * as style from "@destack/style";
import { For } from "@destack/view";

import { palette } from "../../palette.stylex";
import { Favicon, Glyph } from "../figure/glyph";
import { github } from "../tool/github.stylex";
import { notion } from "../tool/notion.stylex";
import { chatgpt, marks, notionTable } from "./rented.stylex";

/** A page narrower than a laptop window, where each tool folds its sidebars away. */
const NARROW = "@container (max-width: 599px)";

/** The system font the rented tools fall back to. */
const system = '-apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif';

/** The typeface each rented tool sets, or the nearest one a machine has. */
const faces = {
    chatgpt: `"Söhne", ui-sans-serif, ${system}`,
    github: `${system}, "Noto Sans"`,
    notion: `ui-sans-serif, ${system}`,
};

/** The assistant's scheduled tasks: title, schedule, the last run's note, and whether it failed. */
const chats: readonly (readonly [
    title: string,
    schedule: string,
    note: string,
    isFailed: boolean,
])[] = [
    ["Pick this week's invite batch", "Mondays at 9:30 AM", "Ran today at 9:38 AM", false],
    ["Draft the invite email", "Mondays at 9:35 AM", "Ran today at 9:39 AM", false],
    ["Check who got invited", "Daily at 6:00 PM", "Can't reach the waitlist database", true],
    ["Summarise launch numbers", "Fridays at 4:00 PM", "Next run Friday", false],
];

/** The assistant's recent chats in its sidebar. */
const recents = ["Invite batch 3", "Waitlist email copy", "Launch checklist", "Pricing page copy"];

/** The notes app's pages under the launch teamspace: emoji and title. */
const notionPages: readonly (readonly [emoji: string, page: string])[] = [
    ["🚀", "Destack launch"],
    ["📋", "Waitlist"],
    ["📝", "Invite email"],
    ["📈", "Launch numbers"],
];

/** The waitlist database's first rows: name, email, whether approved, and when they joined. */
const signups: readonly (readonly [
    name: string,
    email: string,
    isApproved: boolean,
    joined: string,
])[] = [
    ["Ada Park", "ada@hey.com", true, "Oct 2"],
    ["Jonas Weber", "jonas@weber.dev", true, "Oct 3"],
    ["Priya Nair", "priya@nair.io", true, "Oct 3"],
    ["Leo Martin", "leo@martin.fr", true, "Oct 4"],
    ["Sara Lind", "sara@lind.se", false, "Oct 9"],
    ["Tom Okafor", "tom@okafor.co", false, "Oct 11"],
];

/** The database's automations: title, trigger, action, last run, and whether on. */
const automations: readonly (readonly [
    title: string,
    when: string,
    does: string,
    last: string,
    isOn: boolean,
])[] = [
    [
        "Invite approved signups",
        "Status is set to Approved",
        "Send webhook to hooks.symbol.app/invite",
        "Last run today 09:40 by Florian",
        true,
    ],
    ["Post new invites", "Invited is checked", "Send Slack message to #launch", "Never run", true],
];

/** The code host's repository tabs: name, count, and whether open. */
const repoTabs: readonly (readonly [tab: string, count: string, isOn: boolean])[] = [
    ["Code", "", false],
    ["Issues", "12", false],
    ["Pull requests", "3", false],
    ["Actions", "", true],
    ["Projects", "", false],
    ["Settings", "", false],
];

/** The code host's workflows in its sidebar, and whether open. */
const workflowFiles: readonly (readonly [name: string, isOn: boolean])[] = [
    ["All workflows", false],
    ["build", true],
    ["deploy", false],
    ["nightly", false],
];

/** How a job of a run ended. */
type JobState = "passed" | "failed" | "skipped";

/** The jobs of the failed run: name, outcome and duration. */
const jobs: readonly (readonly [name: string, state: JobState, time: string])[] = [
    ["install", "passed", "18s"],
    ["test", "failed", "41s"],
    ["deploy", "skipped", ""],
];

/** The tone of a line of the test job's log. */
type LogTone = "plain" | "passed" | "failed" | "quiet";

/** The test job's log: each line and its tone. */
const testLog: readonly (readonly [text: string, tone: LogTone])[] = [
    ["Run bun test send-invites", "quiet"],
    ["✓ sends one batch of 100 [12.04ms]", "passed"],
    ["✗ waits out the rate limit [41.22s]", "failed"],
    ["  expect(sent).toBe(400)", "plain"],
    ["  Expected: 400", "passed"],
    ["  Received: 230", "failed"],
    ["Error: Process completed with exit code 1.", "failed"],
];

/** Draw the assistant with its scheduled tasks open beside its chat sidebar. */
export function ChatGPTTasks() {
    return (
        <div data-component="ChatGPTPage" {...style.attrs(styles.chatgpt)}>
            {/* sidebar */}
            <nav {...style.attrs(styles.cSidebar)}>
                <span {...style.attrs(styles.cSideTop)}>
                    <Favicon icon="openai" tint={chatgpt.ink} size={18} />
                    <Glyph name="compose" size={16} />
                </span>
                <span {...style.attrs(styles.cSideItem)}>
                    <Glyph name="compose" size={14} />
                    New chat
                </span>
                <span {...style.attrs(styles.cSideItem)}>
                    <Glyph name="search" size={14} />
                    Search chats
                </span>
                <span {...style.attrs(styles.cSideItem)}>
                    <Glyph name="grid" size={14} />
                    GPTs
                </span>
                <span {...style.attrs(styles.cSideItem, styles.cSideOn)}>
                    <Glyph name="clock" size={14} />
                    Tasks
                </span>
                <span {...style.attrs(styles.cSideHead)}>Chats</span>
                <For each={recents}>
                    {(chat) => <span {...style.attrs(styles.cSideItem)}>{chat}</span>}
                </For>
                <span {...style.attrs(styles.cUser)}>
                    <span {...style.attrs(styles.cAvatar)}>F</span>
                    Florian
                    <span {...style.attrs(styles.cMuted)}>Plus</span>
                </span>
            </nav>

            {/* tasks */}
            <main {...style.attrs(styles.cMain)}>
                <span {...style.attrs(styles.cModel)}>
                    ChatGPT <span {...style.attrs(styles.cMuted)}>4o</span>
                    <Glyph name="caret" size={12} />
                </span>
                <div {...style.attrs(styles.cTasks)}>
                    <b {...style.attrs(styles.cTitle)}>Tasks</b>
                    <span {...style.attrs(styles.cTabs)}>
                        <span {...style.attrs(styles.cTab, styles.cTabOn)}>Active</span>
                        <span {...style.attrs(styles.cTab)}>Completed</span>
                    </span>
                    <For each={chats}>
                        {([title, schedule, note, isFailed]) => (
                            <span {...style.attrs(styles.cTask)}>
                                <span {...style.attrs(styles.cTaskIcon)}>
                                    <Glyph name="clock" size={14} />
                                </span>
                                <span {...style.attrs(styles.cTaskText)}>
                                    <b>{title}</b>
                                    <span {...style.attrs(styles.cMuted)}>
                                        {schedule} ·{" "}
                                        <span {...style.attrs(isFailed && styles.cError)}>
                                            {note}
                                        </span>
                                    </span>
                                </span>
                                <Glyph name="more" size={16} />
                            </span>
                        )}
                    </For>
                </div>
            </main>
        </div>
    );
}

/** Draw the notes app's waitlist database with its automations open: approved, never invited. */
export function NotionAutomations() {
    return (
        <div data-component="NotionDatabase" {...style.attrs(styles.notion)}>
            {/* sidebar */}
            <nav {...style.attrs(styles.nSidebar)}>
                <span {...style.attrs(styles.nWorkspace)}>
                    <span {...style.attrs(styles.nWorkspaceMark)}>S</span>
                    symbol
                    <Glyph name="caret" size={12} />
                </span>
                {(
                    [
                        ["search", "Search"],
                        ["grid", "Home"],
                        ["inbox", "Inbox"],
                    ] as const
                ).map(([icon, label]) => (
                    <span {...style.attrs(styles.nSideItem)}>
                        <Glyph name={icon} size={14} />
                        {label}
                    </span>
                ))}
                <span {...style.attrs(styles.nSideHead)}>Launch</span>
                <For each={notionPages}>
                    {([emoji, page]) => (
                        <span
                            {...style.attrs(
                                styles.nSideItem,
                                page === "Waitlist" && styles.nSideOn,
                            )}
                        >
                            <span aria-hidden="true">{emoji}</span>
                            {page}
                        </span>
                    )}
                </For>
            </nav>

            {/* database */}
            <main {...style.attrs(styles.nMain)}>
                <span {...style.attrs(styles.nBar)}>
                    <span {...style.attrs(styles.nMuted)}>🚀 Destack launch /</span>📋 Waitlist
                    <span {...style.attrs(styles.nBarTools)}>
                        Share
                        <Glyph name="star" size={15} />
                        <Glyph name="more" size={15} />
                    </span>
                </span>
                <div {...style.attrs(styles.nPage)}>
                    <b {...style.attrs(styles.nTitle)}>
                        <span aria-hidden="true">📋</span> Waitlist
                    </b>
                    <span {...style.attrs(styles.nComment)}>
                        <span {...style.attrs(styles.nAvatar)}>M</span>
                        <span>
                            <b>Maya</b> <span {...style.attrs(styles.nMuted)}>Today 10:06</span>
                            <br />
                            All 400 say Approved, none say Invited. Did they get the mail?
                        </span>
                    </span>
                    <span {...style.attrs(styles.nViews)}>
                        {["All signups", "Approved", "Invited"].map((view, index) => (
                            <span {...style.attrs(styles.nView, index === 0 && styles.nViewOn)}>
                                {view}
                            </span>
                        ))}
                        <span {...style.attrs(styles.nViewTools)}>
                            <span {...style.attrs(styles.nBolt)}>⚡</span>
                            <Glyph name="filter" size={14} />
                            <Glyph name="search" size={14} />
                            <span {...style.attrs(styles.nNew)}>New</span>
                        </span>
                    </span>
                    <div {...style.attrs(styles.nTable)}>
                        <span {...style.attrs(styles.nRow, styles.nHead)}>
                            <span>Aa Name</span>
                            <span>Email</span>
                            <span>Status</span>
                            <span>Invited</span>
                            <span>Joined</span>
                        </span>
                        <For each={signups}>
                            {([name, email, isApproved, joined]) => (
                                <span {...style.attrs(styles.nRow)}>
                                    <b {...style.attrs(styles.nName)}>{name}</b>
                                    <span {...style.attrs(styles.nMuted)}>{email}</span>
                                    <span>
                                        <span
                                            {...style.attrs(
                                                styles.nTag,
                                                isApproved ? styles.nTagGreen : styles.nTagGray,
                                            )}
                                        >
                                            {isApproved ? "Approved" : "Waiting"}
                                        </span>
                                    </span>
                                    <span>
                                        <span {...style.attrs(styles.nBox)} />
                                    </span>
                                    <span {...style.attrs(styles.nMuted)}>{joined}</span>
                                </span>
                            )}
                        </For>
                        <span {...style.attrs(styles.nCount)}>COUNT 2,240</span>
                    </div>
                </div>
            </main>

            {/* automations */}
            <aside {...style.attrs(styles.nPanel)}>
                <span {...style.attrs(styles.nPanelHead)}>
                    Automations
                    <Glyph name="close" size={14} />
                </span>
                <For each={automations}>
                    {([title, when, does, last, isOn]) => (
                        <span {...style.attrs(styles.nAutomation)}>
                            <span {...style.attrs(styles.nAutomationHead)}>
                                <b>{title}</b>
                                <span {...style.attrs(styles.nToggle, isOn && styles.nToggleOn)}>
                                    <span {...style.attrs(styles.nKnob)} />
                                </span>
                            </span>
                            <span {...style.attrs(styles.nStep)}>
                                <span {...style.attrs(styles.nMuted)}>When</span>
                                {when}
                            </span>
                            <span {...style.attrs(styles.nStep)}>
                                <span {...style.attrs(styles.nMuted)}>Do</span>
                                {does}
                            </span>
                            <span {...style.attrs(styles.nMuted)}>{last}</span>
                        </span>
                    )}
                </For>
                <span {...style.attrs(styles.nMuted)}>+ New automation</span>
            </aside>
        </div>
    );
}

/** Draw the code host's Actions tab on a failed run, with its job graph. */
export function GitHubRun() {
    return (
        <div data-component="GitHubPage" {...style.attrs(styles.github)}>
            {/* repository header */}
            <header {...style.attrs(styles.hHeader)}>
                <span {...style.attrs(styles.hRepo)}>
                    <Favicon icon="github" tint={github.foreground} size={20} />
                    <span {...style.attrs(styles.hMuted)}>symbol /</span>
                    <b>waitlist-app</b>
                </span>
                <span {...style.attrs(styles.hTabs)}>
                    <For each={repoTabs}>
                        {([tab, count, isOn]) => (
                            <span {...style.attrs(styles.hTab, isOn && styles.hTabOn)}>
                                {tab}
                                {count === "" ? undefined : (
                                    <span {...style.attrs(styles.hCount)}>{count}</span>
                                )}
                            </span>
                        )}
                    </For>
                </span>
            </header>

            {/* workflows, then the run */}
            <div {...style.attrs(styles.hBody)}>
                <nav {...style.attrs(styles.hNav)}>
                    <b {...style.attrs(styles.hNavTitle)}>Actions</b>
                    <For each={workflowFiles}>
                        {([name, isOn]) => (
                            <span {...style.attrs(styles.hNavItem, isOn && styles.hNavOn)}>
                                {name}
                            </span>
                        )}
                    </For>
                </nav>
                <main {...style.attrs(styles.hMain)}>
                    <span {...style.attrs(styles.hRunTitle)}>
                        <span {...style.attrs(styles.hFailed)}>
                            <Glyph name="close" size={14} weight={3} />
                        </span>
                        <b>Batch invites through Resend</b>
                        <span {...style.attrs(styles.hMuted)}>deploy #87</span>
                    </span>
                    <span {...style.attrs(styles.hSummary)}>
                        <span {...style.attrs(styles.hFact)}>
                            <span {...style.attrs(styles.hMuted)}>Triggered via push</span>
                            <span>
                                florian pushed <code {...style.attrs(styles.hCode)}>a41f2c9</code>
                            </span>
                        </span>
                        <span {...style.attrs(styles.hFact)}>
                            <span {...style.attrs(styles.hMuted)}>Status</span>
                            <b>Failure</b>
                        </span>
                        <span {...style.attrs(styles.hFact)}>
                            <span {...style.attrs(styles.hMuted)}>Total duration</span>
                            <b>59s</b>
                        </span>
                    </span>
                    <div {...style.attrs(styles.hGraph)}>
                        <span {...style.attrs(styles.hMuted)}>build.yml · on: push</span>
                        <span {...style.attrs(styles.hJobs)}>
                            <For each={jobs}>
                                {([name, state, time], index) => (
                                    <>
                                        {index() > 0 ? (
                                            <span {...style.attrs(styles.hEdge)} />
                                        ) : undefined}
                                        <span {...style.attrs(styles.hJob)}>
                                            <span {...style.attrs(jobState[state])}>
                                                <Glyph
                                                    name={state === "passed" ? "check" : "close"}
                                                    size={12}
                                                    weight={3}
                                                />
                                            </span>
                                            {name}
                                            <span {...style.attrs(styles.hMuted)}>{time}</span>
                                        </span>
                                    </>
                                )}
                            </For>
                        </span>
                    </div>
                    <span {...style.attrs(styles.hAnnotation)}>
                        <b>test</b> · Expected batches to wait out the rate limit ·
                        send-invites.test.ts:42
                    </span>
                    <pre {...style.attrs(styles.hLog)}>
                        {testLog.map(([text, tone]) => (
                            <code {...style.attrs(styles.hLogLine, logTone[tone])}>{text}</code>
                        ))}
                    </pre>
                </main>
            </div>
        </div>
    );
}

/** The tones of the test job's log lines. */
const logTone = style.create({
    plain: {},
    passed: { color: marks.passed },
    failed: { color: marks.danger },
    quiet: { color: github.muted },
});

/** The code host's job outcomes. */
const jobState = style.create({
    passed: { color: marks.passed, display: "inline-flex" },
    failed: { color: marks.danger, display: "inline-flex" },
    skipped: { color: marks.skipped, display: "inline-flex" },
});

/** The rented automation page styles, in the pixels of a real window. */
const styles = style.create({
    // notion
    notion: {
        backgroundColor: notion.surface,
        color: notion.ink,
        display: "grid",
        fontFamily: faces.notion,
        fontSize: "13px",
        gridTemplateColumns: {
            default: "200px minmax(0, 1fr) 260px",
            [NARROW]: "minmax(0, 1fr)",
        },
        height: "100%",
    },
    nSidebar: {
        alignContent: "start",
        backgroundColor: notionTable.sidebar,
        display: { default: "grid", [NARROW]: "none" },
        gap: "1px",
        paddingBlock: "10px",
        paddingInline: "8px",
    },
    nWorkspace: {
        alignItems: "center",
        display: "flex",
        fontWeight: 600,
        gap: "8px",
        paddingBlock: "4px",
        paddingInline: "8px",
        marginBottom: "6px",
    },
    nWorkspaceMark: {
        alignItems: "center",
        backgroundColor: notion.charcoal,
        borderRadius: "4px",
        color: notion.surface,
        display: "inline-flex",
        fontSize: "11px",
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    nSideItem: {
        alignItems: "center",
        borderRadius: "6px",
        color: notionTable.muted,
        display: "flex",
        gap: "8px",
        paddingBlock: "4px",
        paddingInline: "8px",
    },
    nSideOn: {
        backgroundColor: notionTable.hover,
        color: notion.ink,
        fontWeight: 600,
    },
    nSideHead: {
        color: notionTable.muted,
        fontSize: "12px",
        fontWeight: 600,
        paddingBottom: "2px",
        paddingInline: "8px",
        paddingTop: "14px",
    },
    nMain: {
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minWidth: 0,
    },
    nBar: {
        alignItems: "center",
        display: "flex",
        gap: "6px",
        paddingBlock: "10px",
        paddingInline: "14px",
        whiteSpace: "nowrap",
    },
    nBarTools: {
        alignItems: "center",
        color: notionTable.muted,
        display: "flex",
        gap: "12px",
        marginLeft: "auto",
    },
    nMuted: {
        color: notionTable.muted,
    },
    nPage: {
        alignContent: "start",
        display: "grid",
        gap: "12px",
        minWidth: 0,
        overflow: "hidden",
        paddingInline: { default: "48px", [NARROW]: "16px" },
        paddingTop: "18px",
    },
    nTitle: {
        fontSize: "30px",
        fontWeight: 700,
    },
    nComment: {
        borderBottomColor: notionTable.border,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "10px",
        lineHeight: 1.5,
        paddingBottom: "12px",
    },
    nAvatar: {
        alignItems: "center",
        backgroundColor: palette.rust,
        borderRadius: "50%",
        color: marks.white,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "11px",
        fontWeight: 700,
        height: "22px",
        justifyContent: "center",
        width: "22px",
    },
    nViews: {
        alignItems: "center",
        borderBottomColor: notionTable.border,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "4px",
        whiteSpace: "nowrap",
    },
    nView: {
        color: notionTable.muted,
        paddingBlock: "6px",
        paddingInline: "8px",
    },
    nViewOn: {
        boxShadow: `inset 0 -2px 0 ${notion.ink}`,
        color: notion.ink,
        fontWeight: 600,
    },
    nViewTools: {
        alignItems: "center",
        color: notionTable.muted,
        display: { default: "flex", [NARROW]: "none" },
        gap: "12px",
        marginLeft: "auto",
    },
    nBolt: {
        backgroundColor: notionTable.select,
        borderRadius: "4px",
        paddingInline: "4px",
    },
    nNew: {
        backgroundColor: notion.link,
        borderRadius: "6px",
        color: marks.white,
        fontWeight: 600,
        paddingBlock: "3px",
        paddingInline: "10px",
    },
    nTable: {
        display: "grid",
    },
    nRow: {
        alignItems: "center",
        borderBottomColor: notionTable.border,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        columnGap: "12px",
        display: "grid",
        gridTemplateColumns: {
            default: "minmax(0, 1.2fr) minmax(0, 1.4fr) 90px 60px 60px",
            [NARROW]: "minmax(0, 1fr) 90px 60px",
        },
        minHeight: "33px",
        whiteSpace: "nowrap",
    },
    nHead: {
        color: notionTable.muted,
        fontSize: "12px",
    },
    nName: {
        fontWeight: 500,
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    nTag: {
        borderRadius: "4px",
        fontSize: "12px",
        paddingBlock: "1px",
        paddingInline: "6px",
    },
    nTagGreen: {
        backgroundColor: notionTable.green,
        color: notionTable.greenInk,
    },
    nTagGray: {
        backgroundColor: notionTable.hover,
        color: notion.ink,
    },
    nBox: {
        borderColor: notionTable.muted,
        borderRadius: "3px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        display: "inline-block",
        height: "14px",
        width: "14px",
    },
    nCount: {
        color: notionTable.muted,
        fontSize: "11px",
        paddingTop: "8px",
        textAlign: "end",
    },
    nPanel: {
        alignContent: "start",
        borderInlineStartColor: notionTable.border,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
        display: { default: "grid", [NARROW]: "none" },
        gap: "12px",
        paddingBlock: "14px",
        paddingInline: "14px",
    },
    nPanelHead: {
        alignItems: "center",
        display: "flex",
        fontSize: "14px",
        fontWeight: 600,
        justifyContent: "space-between",
    },
    nAutomation: {
        borderColor: notionTable.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        gap: "8px",
        paddingBlock: "10px",
        paddingInline: "12px",
    },
    nAutomationHead: {
        alignItems: "center",
        display: "flex",
        justifyContent: "space-between",
    },
    nToggle: {
        backgroundColor: notionTable.hover,
        borderRadius: "999px",
        display: "inline-flex",
        height: "14px",
        padding: "2px",
        width: "26px",
    },
    nToggleOn: {
        backgroundColor: notion.link,
        justifyContent: "flex-end",
    },
    nKnob: {
        backgroundColor: marks.white,
        borderRadius: "50%",
        height: "10px",
        width: "10px",
    },
    nStep: {
        display: "grid",
        gap: "8px",
        gridTemplateColumns: "36px minmax(0, 1fr)",
    },

    // chatgpt
    chatgpt: {
        backgroundColor: chatgpt.canvas,
        color: chatgpt.ink,
        display: "grid",
        fontFamily: faces.chatgpt,
        fontSize: "14px",
        gridTemplateColumns: { default: "220px minmax(0, 1fr)", [NARROW]: "minmax(0, 1fr)" },
        height: "100%",
    },
    cSidebar: {
        alignContent: "start",
        backgroundColor: chatgpt.sidebar,
        display: { default: "grid", [NARROW]: "none" },
        gap: "2px",
        gridTemplateRows: "repeat(10, max-content) 1fr",
        paddingBlock: "12px",
        paddingInline: "8px",
    },
    cSideTop: {
        alignItems: "center",
        color: chatgpt.muted,
        display: "flex",
        justifyContent: "space-between",
        paddingBottom: "12px",
        paddingInline: "8px",
        paddingTop: "4px",
    },
    cSideItem: {
        alignItems: "center",
        borderRadius: "8px",
        display: "flex",
        gap: "10px",
        overflow: "hidden",
        paddingBlock: "7px",
        paddingInline: "8px",
        whiteSpace: "nowrap",
    },
    cSideOn: {
        backgroundColor: chatgpt.border,
    },
    cSideHead: {
        color: chatgpt.muted,
        fontSize: "12px",
        paddingBottom: "4px",
        paddingInline: "8px",
        paddingTop: "14px",
    },
    cUser: {
        alignItems: "center",
        alignSelf: "end",
        display: "flex",
        gap: "8px",
        paddingInline: "8px",
    },
    cAvatar: {
        alignItems: "center",
        backgroundColor: chatgpt.green,
        borderRadius: "50%",
        color: marks.white,
        display: "inline-flex",
        fontSize: "11px",
        fontWeight: 700,
        height: "24px",
        justifyContent: "center",
        width: "24px",
    },
    cMuted: {
        color: chatgpt.muted,
    },
    cMain: {
        alignContent: "start",
        display: "grid",
        gap: "8px",
        overflow: "hidden",
        paddingBlock: "12px",
        paddingInline: "16px",
    },
    cModel: {
        alignItems: "center",
        display: "flex",
        fontSize: "16px",
        fontWeight: 600,
        gap: "6px",
    },
    cTasks: {
        display: "grid",
        gap: "10px",
        justifySelf: "center",
        maxWidth: "520px",
        paddingTop: "18px",
        width: "100%",
    },
    cTitle: {
        fontSize: "24px",
        fontWeight: 600,
    },
    cTabs: {
        display: "flex",
        gap: "6px",
    },
    cTab: {
        borderRadius: "999px",
        color: chatgpt.muted,
        paddingBlock: "5px",
        paddingInline: "12px",
    },
    cTabOn: {
        backgroundColor: chatgpt.border,
        color: chatgpt.ink,
    },
    cTask: {
        alignItems: "center",
        borderColor: chatgpt.border,
        borderRadius: "14px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: chatgpt.muted,
        display: "flex",
        gap: "12px",
        paddingBlock: "12px",
        paddingInline: "14px",
    },
    cTaskIcon: {
        alignItems: "center",
        backgroundColor: chatgpt.sidebar,
        borderRadius: "50%",
        color: chatgpt.ink,
        display: "inline-flex",
        flexShrink: 0,
        height: "30px",
        justifyContent: "center",
        width: "30px",
    },
    cTaskText: {
        color: chatgpt.ink,
        display: "grid",
        flexGrow: 1,
        fontSize: "13px",
        gap: "2px",
    },
    cError: {
        color: chatgpt.error,
    },
    // github
    github: {
        backgroundColor: github.canvas,
        color: github.foreground,
        display: "grid",
        fontFamily: faces.github,
        fontSize: "14px",
        gridTemplateRows: "auto minmax(0, 1fr)",
        height: "100%",
    },
    hHeader: {
        backgroundColor: github.inset,
        borderBottomColor: github.border,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "grid",
        gap: "12px",
        paddingInline: "18px",
        paddingTop: "14px",
    },
    hRepo: {
        alignItems: "center",
        display: "flex",
        gap: "8px",
    },
    hMuted: {
        color: github.muted,
    },
    hTabs: {
        display: "flex",
        gap: "4px",
        overflow: "hidden",
    },
    hTab: {
        alignItems: "center",
        color: github.foreground,
        display: "flex",
        fontSize: "13px",
        gap: "6px",
        paddingBlock: "8px",
        paddingInline: "10px",
        whiteSpace: "nowrap",
    },
    hTabOn: {
        borderBottomColor: github.attention,
        borderBottomStyle: "solid",
        borderBottomWidth: "2px",
        fontWeight: 600,
    },
    hCount: {
        backgroundColor: github.neutral,
        borderRadius: "999px",
        fontSize: "11px",
        paddingInline: "6px",
    },
    hBody: {
        display: "grid",
        gap: "20px",
        gridTemplateColumns: { default: "170px minmax(0, 1fr)", [NARROW]: "minmax(0, 1fr)" },
        minHeight: 0,
        paddingBlock: "18px",
        paddingInline: "18px",
    },
    hNav: {
        alignContent: "start",
        display: { default: "grid", [NARROW]: "none" },
        gap: "2px",
    },
    hNavTitle: {
        fontSize: "18px",
        paddingBottom: "10px",
    },
    hNavItem: {
        borderRadius: "6px",
        fontSize: "13px",
        paddingBlock: "6px",
        paddingInline: "8px",
    },
    hNavOn: {
        backgroundColor: github.subtle,
        boxShadow: `inset 2px 0 0 ${github.accent}`,
        fontWeight: 600,
    },
    hMain: {
        alignContent: "start",
        display: "grid",
        gap: "14px",
        overflow: "hidden",
    },
    hRunTitle: {
        alignItems: "center",
        display: "flex",
        fontSize: "18px",
        gap: "8px",
    },
    hFailed: {
        alignItems: "center",
        backgroundColor: marks.danger,
        borderRadius: "50%",
        color: marks.white,
        display: "inline-flex",
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    hSummary: {
        borderColor: github.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        fontSize: "12px",
        gap: "28px",
        paddingBlock: "10px",
        paddingInline: "14px",
    },
    hFact: {
        display: "grid",
        gap: "2px",
    },
    hCode: {
        backgroundColor: github.neutral,
        borderRadius: "4px",
        fontSize: "11px",
        paddingInline: "4px",
    },
    hGraph: {
        backgroundColor: github.inset,
        backgroundImage: `radial-gradient(${github.border} 1px, transparent 1.5px)`,
        backgroundSize: "14px 14px",
        borderColor: github.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        fontSize: "12px",
        gap: "14px",
        paddingBlock: "14px",
        paddingInline: "16px",
    },
    hJobs: {
        alignItems: "center",
        display: "flex",
    },
    hJob: {
        alignItems: "center",
        backgroundColor: github.canvas,
        borderColor: github.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        fontSize: "13px",
        gap: "8px",
        paddingBlock: "8px",
        paddingInline: "10px",
    },
    hEdge: {
        backgroundColor: github.border,
        flexGrow: 1,
        height: "2px",
        maxWidth: "40px",
        minWidth: "16px",
    },
    hLog: {
        backgroundColor: github.inset,
        borderColor: github.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        margin: 0,
        paddingBlock: "10px",
        paddingInline: "14px",
    },
    hLogLine: {
        fontFamily: '"SFMono-Regular", ui-monospace, Menlo, monospace',
        fontSize: "12px",
        lineHeight: "20px",
        whiteSpace: "pre",
    },
    hAnnotation: {
        backgroundColor: marks.dangerField,
        borderColor: marks.danger,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        fontSize: "12px",
        paddingBlock: "8px",
        paddingInline: "12px",
    },
});
