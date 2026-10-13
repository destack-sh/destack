import * as style from "@destack/style";
import { For } from "@destack/view";

import { Favicon, Glyph } from "../figure/glyph";
import { github } from "../verb/vendor.stylex";
import { chatgpt, marks } from "./rented.stylex";

/** A page narrower than a laptop window, where each tool folds its sidebars away. */
const NARROW = "@container (max-width: 599px)";

/** The system font the rented tools fall back to. */
const system = '-apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif';

/** The typeface each rented tool sets, or the nearest one a machine has. */
const faces = {
    chatgpt: `"Söhne", ui-sans-serif, ${system}`,
    github: `${system}, "Noto Sans"`,
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
