import * as style from "@destack/style";
import { For, type JSX } from "@destack/view";

import { Favicon, Glyph } from "../figure/glyph";
import { calendly, github, gmail, linear, notion, slack } from "../verb/vendor.stylex";
import { chatgpt, marks, zapier } from "./rented.stylex";

/** A page narrower than a laptop window, where each tool folds its sidebars away. */
const NARROW = "@container (max-width: 599px)";

/** The system font the rented tools fall back to. */
const system = '-apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif';

/** The typeface each rented tool sets, or the nearest one a machine has. */
const faces = {
    zapier: `Inter, ${system}`,
    chatgpt: `"Söhne", ui-sans-serif, ${system}`,
    calendly: `Gilroy, "Proxima Nova", ${system}`,
    linear: `"Inter Variable", Inter, ${system}`,
    slack: `Lato, "Slack-Lato", ${system}`,
    notion: `ui-sans-serif, ${system}`,
    gmail: `"Google Sans", Roboto, ${system}`,
    github: `${system}, "Noto Sans"`,
};

/** How a step of a zap last ran. */
type StepState = "ran" | "failed" | "skipped";

/** The zap's steps in the editor: app, colour, kind, event, app name, and how each ran. */
const steps: readonly (readonly [
    app: string,
    tint: string,
    kind: string,
    event: string,
    name: string,
    state: StepState,
])[] = [
    ["calendly", "#006bff", "1. Trigger", "Invitee Created", "Calendly", "ran"],
    ["granola", "#65a30d", "2. Action", "Create Meeting Note", "Granola", "failed"],
    ["linear", "#5e6ad2", "3. Action", "Create Issue", "Linear", "skipped"],
    ["slack", "#4a154b", "4. Action", "Send Channel Message", "Slack", "skipped"],
];

/** The fields of the failed step's setup, as the editor's step panel lists them. */
const setup: readonly (readonly [field: string, value: string])[] = [
    ["App", "Granola"],
    ["Action event", "Create Meeting Note"],
    ["Account", "florian@symbol.dev"],
];

/** The assistant's scheduled tasks: title, schedule, the last run's note, and whether it failed. */
const chats: readonly (readonly [
    title: string,
    schedule: string,
    note: string,
    isFailed: boolean,
])[] = [
    ["Morning brief", "Daily at 8:00 AM", "Ran today at 8:00 AM", false],
    ["Summarise yesterday's calls", "Daily at 8:05 AM", "Granola isn't connected", true],
    ["Check competitor launches", "Weekdays at 9:00 AM", "Ran today at 9:00 AM", false],
    ["Draft the weekly update", "Fridays at 4:00 PM", "Next run Friday", false],
];

/** The assistant's recent chats in its sidebar. */
const recents = ["Morning brief", "Pricing page copy", "Launch email draft", "Demo script"];

/** The booking tool's navigation: each item, its glyph, and whether it is open. */
const bookingNav: readonly (readonly [
    item: string,
    glyph: "calendar" | "clock" | "person" | "layers" | "grid" | "share",
    isOn: boolean,
])[] = [
    ["Scheduling", "calendar", false],
    ["Meetings", "clock", false],
    ["Availability", "clock", false],
    ["Contacts", "person", false],
    ["Workflows", "layers", true],
    ["Integrations & apps", "grid", false],
    ["Routing", "share", false],
];

/** The booking tool's workflows: name, what it sends and when, its event, and its switch. */
const reminders: readonly (readonly [title: string, when: string, event: string, isOn: boolean])[] =
    [
        ["Email reminder to invitee", "Email · 1 hour before event starts", "Destack demo", true],
        ["Text reminder to invitee", "Text · 24 hours before event starts", "Destack demo", false],
        ["Thank-you email", "Email · 30 minutes after event ends", "Destack demo", true],
        ["Ask to reschedule", "Email · when event is canceled", "All event types", true],
    ];

/** The task tool's settings navigation: each item, whether it heads a group, and whether open. */
const trackerNav: readonly (readonly [item: string, isHeading: boolean, isOn: boolean])[] = [
    ["Account", true, false],
    ["Preferences", false, false],
    ["Profile", false, false],
    ["Notifications", false, false],
    ["Workspace", true, false],
    ["General", false, false],
    ["Members", false, false],
    ["Integrations", false, false],
    ["Destack", true, false],
    ["General", false, false],
    ["Issue statuses", false, false],
    ["Workflow", false, true],
    ["Triage", false, false],
];

/** A tracker status: started, in review, done, or no change. */
type Status = "started" | "review" | "done" | "none";

/** The task tool's git automations: the pull request event and the status it moves issues to. */
const automations: readonly (readonly [event: string, status: string, mark: Status])[] = [
    ["On draft PR open", "In Progress", "started"],
    ["On PR open", "In Progress", "started"],
    ["On PR review request", "In Review", "review"],
    ["On PR ready for merge", "No action", "none"],
    ["On PR merge", "Done", "done"],
];

/** The chat tool's workflow steps: what each does, and whether it failed today. */
const slackSteps: readonly (readonly [title: string, detail: string, isFailed: boolean])[] = [
    ["Send a message to #launch", "🎉 New signup: {{email}} from {{source}}", false],
    ["Add an item to Signups", "List · Email, Source, Joined", false],
    ["Create an issue in Linear", "Connector · Team DST · Title {{email}}", true],
];

/** A docs tool status pill's field. */
type Tone = "notStarted" | "inProgress" | "done";

/** The docs tool's board: each column's status, its pill and its cards with their owners. */
const columns: readonly (readonly [
    status: string,
    tone: Tone,
    cards: readonly (readonly [title: string, owner: string])[],
])[] = [
    [
        "Not started",
        "notStarted",
        [
            ["Open signups", "F"],
            ["Turn on pricing", "F"],
        ],
    ],
    [
        "In progress",
        "inProgress",
        [
            ["Record the demo", "F"],
            ["Draft the launch post", "A"],
        ],
    ],
    ["Done", "done", [["Freeze the pricing page", "F"]]],
];

/** The docs tool's sidebar pages: emoji, title, and whether open. */
const pages: readonly (readonly [emoji: string, title: string, isOn: boolean])[] = [
    ["🚀", "Launch tasks", true],
    ["📄", "Launch plan", false],
    ["🗒️", "Meeting notes", false],
    ["💬", "Feedback", false],
];

/** The mail tool's folders in its sidebar: name, unread count, and glyph. */
const folders: readonly (readonly [
    folder: string,
    count: string,
    glyph: "inbox" | "star" | "clock" | "share" | "file",
])[] = [
    ["Inbox", "12", "inbox"],
    ["Starred", "", "star"],
    ["Snoozed", "", "clock"],
    ["Sent", "", "share"],
    ["Drafts", "3", "file"],
];

/** The mail tool's filters: what each matches, and what it does. */
const filters: readonly (readonly [matches: string, action: string])[] = [
    [
        "from:(stripe.com) subject:(invoice)",
        "Skip Inbox, Apply label “Invoices”, Forward to files@symbol.dev",
    ],
    ["from:(calendly.com)", "Apply label “Bookings”"],
    ["list:(launch.destack.app)", "Star it"],
    ["from:(github.com)", "Apply label “Code”, Mark as read"],
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

/** Draw a tool's own automation page, as the tool itself draws it. */
export function rentedAutomationOf(tool: string): JSX.Element {
    if (tool === "Zapier") {
        return <Zapier />;
    } else if (tool === "ChatGPT") {
        return <ChatGPT />;
    } else if (tool === "Calendly") {
        return <Calendly />;
    } else if (tool === "Linear") {
        return <Linear />;
    } else if (tool === "Slack") {
        return <Slack />;
    } else if (tool === "Notion") {
        return <Notion />;
    } else if (tool === "Gmail") {
        return <Gmail />;
    }

    return <GitHub />;
}

/** Draw the zap editor: the zap's steps on its canvas, the failing one open in its panel. */
function Zapier() {
    return (
        <div data-component="ZapierPage" {...style.attrs(styles.zapier)}>
            {/* top bar */}
            <header {...style.attrs(styles.zBar)}>
                <Glyph name="back" size={16} />
                <ZapierMark />
                <b {...style.attrs(styles.zZap)}>Demo booked → notes and issue</b>
                <span {...style.attrs(styles.zPill)}>Published</span>
                <span {...style.attrs(styles.zMuted, styles.wideOnly)}>Saved 3 days ago</span>
                <span {...style.attrs(styles.zGhost, styles.wideOnly)}>Test run</span>
                <span {...style.attrs(styles.zPublish)}>Publish</span>
            </header>

            {/* tool rail, canvas and step panel */}
            <div {...style.attrs(styles.zBody)}>
                <nav {...style.attrs(styles.zRail)}>
                    <Glyph name="layers" size={16} />
                    <Glyph name="clock" size={16} />
                    <Glyph name="sliders" size={16} />
                    <Glyph name="help" size={16} />
                </nav>
                <div {...style.attrs(styles.zCanvas)}>
                    <For each={steps}>
                        {([app, tint, kind, event, name, state], index) => (
                            <>
                                {index() > 0 ? (
                                    <span {...style.attrs(styles.zLink)}>
                                        <span {...style.attrs(styles.zPlus)}>+</span>
                                    </span>
                                ) : undefined}
                                <div {...style.attrs(styles.zStep, zapState[state])}>
                                    <AppMark icon={app} tint={tint} size={32} />
                                    <span {...style.attrs(styles.zStepText)}>
                                        <span {...style.attrs(styles.zKind)}>{kind}</span>
                                        <b {...style.attrs(styles.zEvent)}>{event}</b>
                                        <span {...style.attrs(styles.zMuted)}>{name}</span>
                                    </span>
                                    {state === "ran" ? (
                                        <span {...style.attrs(styles.zOk)}>
                                            <Glyph name="check" size={12} weight={3} />
                                        </span>
                                    ) : undefined}
                                    {state === "failed" ? (
                                        <span {...style.attrs(styles.zFail)}>!</span>
                                    ) : undefined}
                                </div>
                            </>
                        )}
                    </For>
                </div>
                <aside {...style.attrs(styles.zPanel)}>
                    <span {...style.attrs(styles.zPanelHead)}>
                        <AppMark icon="granola" tint="#65a30d" size={24} />
                        <b>2. Create Meeting Note</b>
                    </span>
                    <span {...style.attrs(styles.zPanelTabs)}>
                        <span {...style.attrs(styles.zPanelTab, styles.zPanelTabOn)}>Setup</span>
                        <span {...style.attrs(styles.zPanelTab)}>Configure</span>
                        <span {...style.attrs(styles.zPanelTab)}>Test</span>
                    </span>
                    <For each={setup}>
                        {([field, value]) => (
                            <span {...style.attrs(styles.zField)}>
                                <span {...style.attrs(styles.zMuted)}>{field}</span>
                                <span {...style.attrs(styles.zInput)}>{value}</span>
                            </span>
                        )}
                    </For>
                    <span {...style.attrs(styles.zError)}>
                        <b>Granola returned 401 Unauthorized.</b> Reconnect the account to resume
                        this Zap.
                    </span>
                    <span {...style.attrs(styles.zReconnect)}>Reconnect account</span>
                </aside>
            </div>
        </div>
    );
}

/** Draw the automation tool's mark: an orange asterisk of four bars. */
function ZapierMark() {
    return (
        <svg aria-hidden="true" viewBox="0 0 24 24" {...style.attrs(styles.zMark)}>
            <path
                d="M12 3.5v17M3.5 12h17M6 6l12 12M18 6 6 18"
                fill="none"
                stroke="currentColor"
                stroke-width="3.4"
                stroke-linecap="round"
            />
        </svg>
    );
}

/** Draw the assistant with its scheduled tasks open beside its chat sidebar. */
function ChatGPT() {
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

/** Draw the booking tool's workflows beside its navigation. */
function Calendly() {
    return (
        <div data-component="CalendlyPage" {...style.attrs(styles.calendly)}>
            {/* navigation */}
            <nav {...style.attrs(styles.kNav)}>
                <span {...style.attrs(styles.kLogo)}>
                    <Favicon icon="calendly" tint={calendly.blue} size={20} />
                    Calendly
                </span>
                <span {...style.attrs(styles.kCreate)}>+ Create</span>
                <For each={bookingNav}>
                    {([item, glyph, isOn]) => (
                        <span {...style.attrs(styles.kNavItem, isOn && styles.kNavOn)}>
                            <Glyph name={glyph} size={15} />
                            {item}
                        </span>
                    )}
                </For>
            </nav>

            {/* workflows */}
            <main {...style.attrs(styles.kMain)}>
                <span {...style.attrs(styles.kHead)}>
                    <b {...style.attrs(styles.kTitle)}>Workflows</b>
                    <span {...style.attrs(styles.kButton)}>+ Create workflow</span>
                </span>
                <span {...style.attrs(styles.kTabs)}>
                    <span {...style.attrs(styles.kTab, styles.kTabOn)}>My workflows</span>
                    <span {...style.attrs(styles.kTab)}>Templates</span>
                </span>
                <div {...style.attrs(styles.kList)}>
                    <For each={reminders}>
                        {([title, when, event, isOn]) => (
                            <span {...style.attrs(styles.kRow)}>
                                <span {...style.attrs(styles.kRowIcon)}>
                                    <Glyph name="mail" size={15} />
                                </span>
                                <span {...style.attrs(styles.kRowText)}>
                                    <b>{title}</b>
                                    <span {...style.attrs(styles.kMuted)}>{when}</span>
                                </span>
                                <span {...style.attrs(styles.kEvent, styles.wideOnly)}>
                                    {event}
                                </span>
                                <span {...style.attrs(styles.kToggle, isOn && styles.kToggleOn)}>
                                    <span {...style.attrs(styles.kKnob, isOn && styles.kKnobOn)} />
                                </span>
                            </span>
                        )}
                    </For>
                </div>
                <span {...style.attrs(styles.kNote)}>
                    1 email bounced today · maya@northwind.io
                </span>
            </main>
        </div>
    );
}

/** Draw the task tool's team workflow settings with its git automations. */
function Linear() {
    return (
        <div data-component="LinearPage" {...style.attrs(styles.linear)}>
            {/* settings navigation */}
            <nav {...style.attrs(styles.lNav)}>
                <span {...style.attrs(styles.lBack)}>
                    <Glyph name="back" size={13} />
                    Back to app
                </span>
                <For each={trackerNav}>
                    {([item, isHeading, isOn]) => (
                        <span
                            {...style.attrs(
                                isHeading ? styles.lNavHead : styles.lNavItem,
                                isOn && styles.lNavOn,
                            )}
                        >
                            {item}
                        </span>
                    )}
                </For>
            </nav>

            {/* workflow settings */}
            <main {...style.attrs(styles.lMain)}>
                <span {...style.attrs(styles.lCrumb)}>Destack › Workflow</span>
                <b {...style.attrs(styles.lTitle)}>Workflow</b>
                <b {...style.attrs(styles.lSection)}>Git automations</b>
                <span {...style.attrs(styles.lMuted)}>
                    Move issues when their linked pull requests or commits change.
                </span>
                <div {...style.attrs(styles.lTable)}>
                    <span {...style.attrs(styles.lRow, styles.lHeadRow)}>
                        <span>Pull request</span>
                        <span>Default</span>
                    </span>
                    <For each={automations}>
                        {([event, status, mark]) => (
                            <span {...style.attrs(styles.lRow)}>
                                <span>{event}</span>
                                <span {...style.attrs(styles.lSelect)}>
                                    <span {...style.attrs(styles.status, statusMark[mark])} />
                                    {status}
                                    <Glyph name="caret" size={11} />
                                </span>
                            </span>
                        )}
                    </For>
                </div>
                <span {...style.attrs(styles.lMuted)}>+ Add branch rule</span>
            </main>
        </div>
    );
}

/** Draw the chat tool's Workflow Builder with one workflow's trigger and steps. */
function Slack() {
    return (
        <div data-component="SlackPage" {...style.attrs(styles.slack)}>
            {/* workspace bar */}
            <header {...style.attrs(styles.sBar)}>
                <span {...style.attrs(styles.sSearch)}>
                    <Glyph name="search" size={12} />
                    Search Destack
                </span>
            </header>

            {/* rail, then the builder */}
            <div {...style.attrs(styles.sBody)}>
                <nav {...style.attrs(styles.sRail)}>
                    <span {...style.attrs(styles.sTeam)}>D</span>
                    <Glyph name="grid" size={16} />
                    <Glyph name="mail" size={16} />
                    <Glyph name="status" size={16} />
                    <Glyph name="more" size={16} />
                </nav>
                <main {...style.attrs(styles.sMain)}>
                    <span {...style.attrs(styles.sHead)}>
                        <span {...style.attrs(styles.sEmoji)}>👋</span>
                        <b {...style.attrs(styles.sTitle)}>New signup to #launch</b>
                        <span {...style.attrs(styles.sBadge)}>Published</span>
                        <span {...style.attrs(styles.sEdit)}>Edit</span>
                    </span>
                    <div {...style.attrs(styles.sFlow)}>
                        <span {...style.attrs(styles.sLabel)}>Starts when</span>
                        <span {...style.attrs(styles.sCard, styles.sTrigger)}>
                            <span {...style.attrs(styles.sIcon)}>
                                <Glyph name="link" size={14} />
                            </span>
                            <span {...style.attrs(styles.sCardText)}>
                                <b>A webhook receives a request</b>
                                <span {...style.attrs(styles.sMuted)}>
                                    From your waitlist form · 4 today
                                </span>
                            </span>
                        </span>
                        <span {...style.attrs(styles.sLabel)}>Then, do these things</span>
                        <For each={slackSteps}>
                            {([title, detail, isFailed], index) => (
                                <span
                                    {...style.attrs(styles.sCard, isFailed && styles.sCardFailed)}
                                >
                                    <span {...style.attrs(styles.sNumber)}>{index() + 1}</span>
                                    <span {...style.attrs(styles.sCardText)}>
                                        <b>{title}</b>
                                        <span {...style.attrs(styles.sMuted)}>{detail}</span>
                                        {isFailed ? (
                                            <span {...style.attrs(styles.sFailed)}>
                                                Failed once today · reconnect Linear
                                            </span>
                                        ) : undefined}
                                    </span>
                                </span>
                            )}
                        </For>
                    </div>
                </main>
            </div>
        </div>
    );
}

/** Draw the docs tool's task board with one database automation open over it. */
function Notion() {
    return (
        <div data-component="NotionPage" {...style.attrs(styles.notion)}>
            {/* sidebar */}
            <nav {...style.attrs(styles.nSidebar)}>
                <span {...style.attrs(styles.nWorkspace)}>
                    <span {...style.attrs(styles.nTeam)}>D</span>
                    Destack
                </span>
                <span {...style.attrs(styles.nSideItem)}>
                    <Glyph name="search" size={13} />
                    Search
                </span>
                <span {...style.attrs(styles.nSideItem)}>
                    <Glyph name="inbox" size={13} />
                    Inbox
                </span>
                <span {...style.attrs(styles.nSideHead)}>Teamspace</span>
                <For each={pages}>
                    {([emoji, title, isOn]) => (
                        <span {...style.attrs(styles.nSideItem, isOn && styles.nSideOn)}>
                            <span>{emoji}</span>
                            {title}
                        </span>
                    )}
                </For>
            </nav>

            {/* board, with the automation over it */}
            <main {...style.attrs(styles.nMain)}>
                <span {...style.attrs(styles.nTop)}>
                    <span {...style.attrs(styles.nMuted)}>Destack / Launch tasks</span>
                    <span {...style.attrs(styles.nMuted)}>Share</span>
                </span>
                <b {...style.attrs(styles.nTitle)}>🚀 Launch tasks</b>
                <span {...style.attrs(styles.nViews)}>
                    <span {...style.attrs(styles.nView, styles.nViewOn)}>Board</span>
                    <span {...style.attrs(styles.nView)}>Table</span>
                    <span {...style.attrs(styles.nTools)}>
                        <span {...style.attrs(styles.nBolt)}>⚡ 3</span>
                        <span {...style.attrs(styles.nNew)}>New</span>
                    </span>
                </span>
                <div {...style.attrs(styles.nBoard)}>
                    <For each={columns}>
                        {([status, tone, cards]) => (
                            <div {...style.attrs(styles.nColumn)}>
                                <span {...style.attrs(styles.nPill, notionTone[tone])}>
                                    {status}
                                </span>
                                <For each={cards}>
                                    {([title, owner]) => (
                                        <span {...style.attrs(styles.nCard)}>
                                            <b>{title}</b>
                                            <span {...style.attrs(styles.nOwner)}>{owner}</span>
                                        </span>
                                    )}
                                </For>
                            </div>
                        )}
                    </For>
                </div>

                {/* automation editor */}
                <div {...style.attrs(styles.nPanel)}>
                    <b {...style.attrs(styles.nPanelTitle)}>⚡ Status change</b>
                    <span {...style.attrs(styles.nPanelHead)}>When</span>
                    <span {...style.attrs(styles.nRule)}>
                        Status <span {...style.attrs(styles.nMuted)}>is set to</span>
                        <span {...style.attrs(styles.nPill, notionTone.done)}>Done</span>
                    </span>
                    <span {...style.attrs(styles.nPanelHead)}>Do</span>
                    <span {...style.attrs(styles.nRule)}>
                        <Favicon icon="slack" tint={marks.berry} size={13} />
                        Send Slack notification to <b>#launch</b>
                    </span>
                    <span {...style.attrs(styles.nRule)}>
                        Edit property <b>Done date</b>
                        <span {...style.attrs(styles.nMuted)}>→ Now</span>
                    </span>
                    <span {...style.attrs(styles.nPanelFoot)}>
                        <span {...style.attrs(styles.nMuted)}>Ran 5 times today</span>
                        <span {...style.attrs(styles.nSave)}>Save</span>
                    </span>
                </div>
            </main>
        </div>
    );
}

/** Draw the mail tool's settings on its filters tab, inside its own chrome. */
function Gmail() {
    return (
        <div data-component="GmailPage" {...style.attrs(styles.gmail)}>
            {/* top bar */}
            <header {...style.attrs(styles.gBar)}>
                <Glyph name="menu" size={16} />
                <span {...style.attrs(styles.gLogo)}>
                    <Favicon icon="gmail" tint={marks.gmailRed} size={20} />
                    Gmail
                </span>
                <span {...style.attrs(styles.gSearch)}>
                    <Glyph name="search" size={15} />
                    Search mail
                    <span {...style.attrs(styles.gPush)} />
                    <Glyph name="sliders" size={15} />
                </span>
                <span {...style.attrs(styles.gAvatar)}>F</span>
            </header>

            {/* folders, then settings */}
            <div {...style.attrs(styles.gBody)}>
                <nav {...style.attrs(styles.gNav)}>
                    <span {...style.attrs(styles.gCompose)}>
                        <Glyph name="compose" size={16} />
                        Compose
                    </span>
                    <For each={folders}>
                        {([folder, count, glyph]) => (
                            <span {...style.attrs(styles.gFolder)}>
                                <Glyph name={glyph} size={15} />
                                {folder}
                                <span {...style.attrs(styles.gCount)}>{count}</span>
                            </span>
                        )}
                    </For>
                </nav>
                <main {...style.attrs(styles.gMain)}>
                    <b {...style.attrs(styles.gTitle)}>Settings</b>
                    <span {...style.attrs(styles.gTabs)}>
                        <span {...style.attrs(styles.gTab)}>General</span>
                        <span {...style.attrs(styles.gTab)}>Labels</span>
                        <span {...style.attrs(styles.gTab)}>Inbox</span>
                        <span {...style.attrs(styles.gTab, styles.gTabOn)}>
                            Filters and Blocked Addresses
                        </span>
                        <span {...style.attrs(styles.gTab, styles.wideOnly)}>Forwarding</span>
                    </span>
                    <span {...style.attrs(styles.gNote)}>
                        The following filters are applied to all incoming mail:
                    </span>
                    <For each={filters}>
                        {([matches, action]) => (
                            <span {...style.attrs(styles.gFilter)}>
                                <span {...style.attrs(styles.gCheck)} />
                                <span {...style.attrs(styles.gFilterText)}>
                                    <span>
                                        <b>Matches:</b> {matches}
                                    </span>
                                    <span>
                                        <b>Do this:</b> {action}
                                    </span>
                                </span>
                                <span {...style.attrs(styles.gLinks)}>edit · delete</span>
                            </span>
                        )}
                    </For>
                    <span {...style.attrs(styles.gNew)}>Create a new filter · Import filters</span>
                </main>
            </div>
        </div>
    );
}

/** Draw the code host's Actions tab on a failed run, with its job graph. */
function GitHub() {
    return (
        <div data-component="GitHubPage" {...style.attrs(styles.github)}>
            {/* repository header */}
            <header {...style.attrs(styles.hHeader)}>
                <span {...style.attrs(styles.hRepo)}>
                    <Favicon icon="github" tint={github.foreground} size={20} />
                    <span {...style.attrs(styles.hMuted)}>destack /</span>
                    <b>site</b>
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
                        <b>Show yearly prices</b>
                        <span {...style.attrs(styles.hMuted)}>build #411</span>
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
                        <b>test</b> · Expected the yearly price to render · prices.test.ts:42
                    </span>
                </main>
            </div>
        </div>
    );
}

/** Draw an app's mark on a small white tile: its logo where it only has one, or its icon. */
function AppMark(properties: { icon: string; tint: string; size: number }) {
    return (
        <span
            style={{
                width: `${String(properties.size)}px`,
                height: `${String(properties.size)}px`,
            }}
            {...style.attrs(styles.appMark)}
        >
            {properties.icon === "granola" ? (
                <img alt="" src="/logos/granola.png" {...style.attrs(styles.appLogo)} />
            ) : (
                <Favicon icon={properties.icon} tint={properties.tint} size={properties.size - 8} />
            )}
        </span>
    );
}

/** The step cards of the zap editor by how each last ran. */
const zapState = style.create({
    ran: {},
    failed: {
        borderColor: zapier.orange,
        boxShadow: `0 0 0 3px color-mix(in srgb, ${zapier.orange} 22%, transparent)`,
    },
    skipped: {
        color: zapier.skipped,
        opacity: 0.7,
    },
});

/** The fields of the docs tool's status pills. */
const notionTone = style.create({
    notStarted: { backgroundColor: marks.notStarted },
    inProgress: { backgroundColor: marks.inProgress },
    done: { backgroundColor: marks.done },
});

/** The tracker status marks. */
const statusMark = style.create({
    started: {
        backgroundImage: `conic-gradient(${marks.amber} 0 50%, transparent 50%)`,
        borderColor: marks.amber,
    },
    review: { borderColor: marks.green },
    done: { backgroundColor: marks.violet, borderColor: marks.violet },
    none: { borderColor: linear.muted, borderStyle: "dashed" },
});

/** The code host's job outcomes. */
const jobState = style.create({
    passed: { color: marks.passed, display: "inline-flex" },
    failed: { color: marks.danger, display: "inline-flex" },
    skipped: { color: marks.skipped, display: "inline-flex" },
});

/** The rented automation page styles, in the pixels of a real window. */
const styles = style.create({
    wideOnly: {
        display: { default: "inline-flex", [NARROW]: "none" },
    },
    appMark: {
        alignItems: "center",
        backgroundColor: marks.white,
        borderRadius: "6px",
        boxShadow: `0 0 0 1px ${zapier.border}`,
        display: "inline-flex",
        flexShrink: 0,
        justifyContent: "center",
        overflow: "hidden",
    },
    appLogo: {
        height: "100%",
        width: "100%",
    },
    status: {
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1.5px",
        boxSizing: "border-box",
        display: "inline-block",
        flexShrink: 0,
        height: "12px",
        width: "12px",
    },

    // zapier
    zapier: {
        backgroundColor: zapier.canvas,
        color: zapier.ink,
        display: "grid",
        fontFamily: faces.zapier,
        fontSize: "13px",
        gridTemplateRows: "52px minmax(0, 1fr)",
        height: "100%",
    },
    zBar: {
        alignItems: "center",
        backgroundColor: zapier.card,
        borderBottomColor: zapier.border,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: zapier.muted,
        display: "flex",
        gap: "12px",
        paddingInline: "16px",
    },
    zMark: {
        color: zapier.orange,
        flexShrink: 0,
        height: "20px",
        width: "20px",
    },
    zZap: {
        color: zapier.ink,
        fontSize: "14px",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    zPill: {
        backgroundColor: zapier.successField,
        borderRadius: "999px",
        color: zapier.success,
        fontSize: "11px",
        fontWeight: 600,
        marginRight: "auto",
        paddingBlock: "2px",
        paddingInline: "8px",
    },
    zMuted: {
        color: zapier.muted,
        fontSize: "12px",
    },
    zGhost: {
        borderColor: zapier.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: zapier.ink,
        fontWeight: 600,
        paddingBlock: "6px",
        paddingInline: "12px",
    },
    zPublish: {
        backgroundColor: zapier.orange,
        borderRadius: "6px",
        color: marks.white,
        fontWeight: 600,
        paddingBlock: "7px",
        paddingInline: "14px",
    },
    zBody: {
        display: "grid",
        gridTemplateColumns: {
            default: "48px minmax(0, 1fr) 280px",
            [NARROW]: "minmax(0, 1fr)",
        },
        minHeight: 0,
    },
    zRail: {
        alignItems: "center",
        backgroundColor: zapier.card,
        borderRightColor: zapier.border,
        borderRightStyle: "solid",
        borderRightWidth: "1px",
        color: zapier.muted,
        display: { default: "flex", [NARROW]: "none" },
        flexDirection: "column",
        gap: "20px",
        paddingTop: "18px",
    },
    zCanvas: {
        alignContent: "start",
        backgroundImage: `radial-gradient(${zapier.dot} 1px, transparent 1.5px)`,
        backgroundSize: "16px 16px",
        display: "grid",
        justifyContent: "center",
        justifyItems: "center",
        overflow: "hidden",
        paddingBlock: "24px",
        paddingInline: "16px",
    },
    zStep: {
        alignItems: "center",
        backgroundColor: zapier.card,
        borderColor: zapier.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        boxShadow: `0 1px 2px ${marks.shadow}`,
        boxSizing: "border-box",
        display: "flex",
        gap: "12px",
        paddingBlock: "10px",
        paddingInline: "12px",
        width: { default: "300px", [NARROW]: "100%" },
    },
    zStepText: {
        display: "grid",
        flexGrow: 1,
        gap: "1px",
        minWidth: 0,
    },
    zKind: {
        color: zapier.muted,
        fontSize: "11px",
    },
    zEvent: {
        fontSize: "14px",
    },
    zOk: {
        alignItems: "center",
        backgroundColor: zapier.successField,
        borderRadius: "50%",
        color: zapier.success,
        display: "inline-flex",
        flexShrink: 0,
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    zFail: {
        alignItems: "center",
        backgroundColor: zapier.error,
        borderRadius: "50%",
        color: marks.white,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "12px",
        fontWeight: 800,
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    zLink: {
        alignItems: "center",
        backgroundImage: `linear-gradient(${zapier.border}, ${zapier.border})`,
        backgroundPosition: "center",
        backgroundRepeat: "no-repeat",
        backgroundSize: "2px 100%",
        display: "flex",
        height: "28px",
        justifyContent: "center",
    },
    zPlus: {
        alignItems: "center",
        backgroundColor: zapier.card,
        borderColor: zapier.border,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1px",
        color: zapier.muted,
        display: "inline-flex",
        fontSize: "13px",
        height: "18px",
        justifyContent: "center",
        width: "18px",
    },
    zPanel: {
        alignContent: "start",
        backgroundColor: zapier.card,
        borderLeftColor: zapier.border,
        borderLeftStyle: "solid",
        borderLeftWidth: "1px",
        display: { default: "grid", [NARROW]: "none" },
        gap: "12px",
        paddingBlock: "16px",
        paddingInline: "16px",
    },
    zPanelHead: {
        alignItems: "center",
        display: "flex",
        fontSize: "14px",
        gap: "8px",
    },
    zPanelTabs: {
        borderBottomColor: zapier.border,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "16px",
    },
    zPanelTab: {
        color: zapier.muted,
        paddingBottom: "8px",
    },
    zPanelTabOn: {
        borderBottomColor: zapier.orange,
        borderBottomStyle: "solid",
        borderBottomWidth: "2px",
        color: zapier.ink,
        fontWeight: 600,
    },
    zField: {
        display: "grid",
        gap: "4px",
    },
    zInput: {
        borderColor: zapier.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        paddingBlock: "7px",
        paddingInline: "10px",
    },
    zError: {
        backgroundColor: zapier.errorField,
        borderRadius: "6px",
        color: zapier.error,
        fontSize: "12px",
        lineHeight: 1.45,
        paddingBlock: "8px",
        paddingInline: "10px",
    },
    zReconnect: {
        backgroundColor: zapier.ink,
        borderRadius: "6px",
        color: zapier.card,
        fontWeight: 600,
        paddingBlock: "8px",
        textAlign: "center",
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

    // calendly
    calendly: {
        backgroundColor: calendly.mist,
        color: calendly.ink,
        display: "grid",
        fontFamily: faces.calendly,
        fontSize: "14px",
        gridTemplateColumns: { default: "210px minmax(0, 1fr)", [NARROW]: "minmax(0, 1fr)" },
        height: "100%",
    },
    kNav: {
        alignContent: "start",
        backgroundColor: calendly.surface,
        borderRightColor: marks.border,
        borderRightStyle: "solid",
        borderRightWidth: "1px",
        display: { default: "grid", [NARROW]: "none" },
        gap: "2px",
        paddingBlock: "16px",
        paddingInline: "12px",
    },
    kLogo: {
        alignItems: "center",
        color: calendly.blue,
        display: "flex",
        fontSize: "18px",
        fontWeight: 700,
        gap: "8px",
        paddingBottom: "14px",
        paddingInline: "8px",
    },
    kCreate: {
        borderColor: calendly.blue,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: calendly.blue,
        fontWeight: 600,
        marginBottom: "12px",
        paddingBlock: "7px",
        textAlign: "center",
    },
    kNavItem: {
        alignItems: "center",
        borderRadius: "8px",
        color: calendly.navy,
        display: "flex",
        gap: "10px",
        paddingBlock: "8px",
        paddingInline: "10px",
    },
    kNavOn: {
        backgroundColor: marks.selected,
        color: calendly.deepBlue,
        fontWeight: 600,
    },
    kMain: {
        alignContent: "start",
        display: "grid",
        gap: "14px",
        overflow: "hidden",
        paddingBlock: "24px",
        paddingInline: "28px",
    },
    kHead: {
        alignItems: "center",
        display: "flex",
        justifyContent: "space-between",
    },
    kTitle: {
        color: calendly.navy,
        fontSize: "24px",
    },
    kButton: {
        backgroundColor: calendly.blue,
        borderRadius: "999px",
        color: marks.white,
        fontWeight: 600,
        paddingBlock: "8px",
        paddingInline: "16px",
    },
    kTabs: {
        borderBottomColor: marks.border,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "22px",
    },
    kTab: {
        color: calendly.slate,
        paddingBottom: "10px",
    },
    kTabOn: {
        borderBottomColor: calendly.blue,
        borderBottomStyle: "solid",
        borderBottomWidth: "3px",
        color: calendly.navy,
        fontWeight: 600,
    },
    kList: {
        backgroundColor: calendly.surface,
        borderColor: marks.border,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
    },
    kRow: {
        alignItems: "center",
        borderTopColor: marks.border,
        borderTopStyle: "solid",
        borderTopWidth: { default: "1px", ":first-child": 0 },
        display: "flex",
        gap: "14px",
        paddingBlock: "13px",
        paddingInline: "16px",
    },
    kRowIcon: {
        alignItems: "center",
        backgroundColor: marks.selected,
        borderRadius: "8px",
        color: calendly.blue,
        display: "inline-flex",
        flexShrink: 0,
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },
    kRowText: {
        display: "grid",
        flexGrow: 1,
        gap: "2px",
    },
    kMuted: {
        color: calendly.slate,
        fontSize: "13px",
    },
    kEvent: {
        backgroundColor: calendly.mist,
        borderRadius: "999px",
        color: calendly.navy,
        fontSize: "12px",
        paddingBlock: "4px",
        paddingInline: "10px",
    },
    kToggle: {
        backgroundColor: marks.border,
        borderRadius: "999px",
        boxSizing: "border-box",
        display: "inline-flex",
        flexShrink: 0,
        height: "20px",
        padding: "2px",
        width: "36px",
    },
    kToggleOn: {
        backgroundColor: calendly.blue,
        justifyContent: "flex-end",
    },
    kKnob: {
        backgroundColor: marks.white,
        borderRadius: "50%",
        height: "16px",
        width: "16px",
    },
    kKnobOn: {
        boxShadow: `0 1px 2px ${marks.shadow}`,
    },
    kNote: {
        color: marks.danger,
        fontSize: "13px",
    },

    // linear
    linear: {
        backgroundColor: linear.panel,
        color: linear.ink,
        display: "grid",
        fontFamily: faces.linear,
        fontSize: "13px",
        gridTemplateColumns: { default: "200px minmax(0, 1fr)", [NARROW]: "minmax(0, 1fr)" },
        height: "100%",
    },
    lNav: {
        alignContent: "start",
        display: { default: "grid", [NARROW]: "none" },
        gap: "1px",
        paddingBlock: "14px",
        paddingInline: "12px",
    },
    lBack: {
        alignItems: "center",
        color: linear.muted,
        display: "flex",
        gap: "6px",
        paddingBottom: "8px",
        paddingInline: "8px",
    },
    lNavHead: {
        color: linear.muted,
        fontSize: "12px",
        fontWeight: 500,
        paddingBottom: "4px",
        paddingInline: "8px",
        paddingTop: "12px",
    },
    lNavItem: {
        borderRadius: "6px",
        color: linear.text,
        paddingBlock: "5px",
        paddingInline: "8px",
    },
    lNavOn: {
        backgroundColor: linear.ring,
        color: linear.ink,
        fontWeight: 500,
    },
    lMain: {
        alignContent: "start",
        backgroundColor: linear.surface,
        borderColor: linear.border,
        borderStartStartRadius: { default: "10px", [NARROW]: 0 },
        borderStyle: "solid",
        borderWidth: { default: "1px 0 0 1px", [NARROW]: 0 },
        display: "grid",
        gap: "10px",
        marginTop: { default: "8px", [NARROW]: 0 },
        overflow: "hidden",
        paddingBlock: "28px",
        paddingInline: "36px",
    },
    lCrumb: {
        color: linear.muted,
        fontSize: "12px",
    },
    lTitle: {
        fontSize: "22px",
        fontWeight: 600,
        paddingBottom: "10px",
    },
    lSection: {
        fontSize: "14px",
        fontWeight: 600,
    },
    lMuted: {
        color: linear.muted,
    },
    lTable: {
        borderColor: linear.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
    },
    lRow: {
        alignItems: "center",
        borderTopColor: linear.divider,
        borderTopStyle: "solid",
        borderTopWidth: { default: "1px", ":first-child": 0 },
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr) 160px",
        paddingBlock: "9px",
        paddingInline: "14px",
    },
    lHeadRow: {
        color: linear.muted,
        fontSize: "12px",
    },
    lSelect: {
        alignItems: "center",
        borderColor: linear.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        gap: "7px",
        justifySelf: "start",
        paddingBlock: "3px",
        paddingInline: "8px",
    },

    // slack
    slack: {
        backgroundColor: slack.aubergine,
        color: slack.ink,
        display: "grid",
        fontFamily: faces.slack,
        fontSize: "14px",
        gridTemplateRows: "40px minmax(0, 1fr)",
        height: "100%",
    },
    sBar: {
        alignItems: "center",
        display: "flex",
        justifyContent: "center",
    },
    sSearch: {
        alignItems: "center",
        backgroundColor: marks.glass,
        borderRadius: "6px",
        color: marks.white,
        display: "flex",
        fontSize: "12px",
        gap: "8px",
        paddingBlock: "5px",
        paddingInline: "12px",
        width: "45%",
    },
    sBody: {
        display: "grid",
        gridTemplateColumns: { default: "56px minmax(0, 1fr)", [NARROW]: "minmax(0, 1fr)" },
        minHeight: 0,
    },
    sRail: {
        alignItems: "center",
        color: marks.white,
        display: { default: "flex", [NARROW]: "none" },
        flexDirection: "column",
        gap: "20px",
        paddingTop: "10px",
    },
    sTeam: {
        alignItems: "center",
        backgroundColor: marks.white,
        borderRadius: "8px",
        color: slack.aubergine,
        display: "inline-flex",
        fontWeight: 800,
        height: "34px",
        justifyContent: "center",
        width: "34px",
    },
    sMain: {
        alignContent: "start",
        backgroundColor: slack.surface,
        borderStartStartRadius: { default: "10px", [NARROW]: 0 },
        display: "grid",
        gap: "16px",
        overflow: "hidden",
        paddingBlock: "18px",
        paddingInline: "24px",
    },
    sHead: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
    },
    sEmoji: {
        fontSize: "20px",
    },
    sTitle: {
        fontSize: "18px",
        fontWeight: 900,
    },
    sBadge: {
        backgroundColor: slack.mention,
        borderRadius: "4px",
        color: slack.link,
        fontSize: "12px",
        fontWeight: 700,
        paddingBlock: "2px",
        paddingInline: "6px",
    },
    sEdit: {
        borderColor: slack.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        fontWeight: 700,
        marginLeft: "auto",
        paddingBlock: "4px",
        paddingInline: "12px",
    },
    sFlow: {
        display: "grid",
        gap: "8px",
        justifySelf: "center",
        maxWidth: "460px",
        width: "100%",
    },
    sLabel: {
        color: slack.muted,
        fontSize: "12px",
        fontWeight: 700,
        paddingTop: "6px",
    },
    sCard: {
        alignItems: "flex-start",
        borderColor: slack.line,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        boxShadow: `0 1px 2px ${marks.shadow}`,
        display: "flex",
        gap: "12px",
        paddingBlock: "12px",
        paddingInline: "14px",
    },
    sTrigger: {
        backgroundColor: marks.trigger,
    },
    sCardFailed: {
        borderColor: slack.noticeInk,
    },
    sIcon: {
        alignItems: "center",
        backgroundColor: slack.link,
        borderRadius: "6px",
        color: marks.white,
        display: "inline-flex",
        flexShrink: 0,
        height: "28px",
        justifyContent: "center",
        width: "28px",
    },
    sNumber: {
        alignItems: "center",
        backgroundColor: marks.berry,
        borderRadius: "6px",
        color: marks.white,
        display: "inline-flex",
        flexShrink: 0,
        fontWeight: 900,
        height: "28px",
        justifyContent: "center",
        width: "28px",
    },
    sCardText: {
        display: "grid",
        gap: "3px",
    },
    sMuted: {
        color: slack.muted,
        fontSize: "13px",
    },
    sFailed: {
        backgroundColor: slack.notice,
        borderRadius: "4px",
        color: slack.noticeInk,
        fontSize: "12px",
        justifySelf: "start",
        marginTop: "4px",
        paddingBlock: "3px",
        paddingInline: "6px",
    },

    // notion
    notion: {
        backgroundColor: notion.surface,
        color: notion.ink,
        display: "grid",
        fontFamily: faces.notion,
        fontSize: "14px",
        gridTemplateColumns: { default: "200px minmax(0, 1fr)", [NARROW]: "minmax(0, 1fr)" },
        height: "100%",
    },
    nSidebar: {
        alignContent: "start",
        backgroundColor: marks.column,
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
        paddingBottom: "8px",
        paddingInline: "8px",
    },
    nTeam: {
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
        color: marks.grey,
        display: "flex",
        fontSize: "13px",
        gap: "8px",
        paddingBlock: "4px",
        paddingInline: "8px",
    },
    nSideOn: {
        backgroundColor: notion.sand,
        color: notion.ink,
        fontWeight: 500,
    },
    nSideHead: {
        color: marks.grey,
        fontSize: "12px",
        paddingBottom: "4px",
        paddingInline: "8px",
        paddingTop: "14px",
    },
    nMain: {
        alignContent: "start",
        display: "grid",
        gap: "10px",
        overflow: "hidden",
        paddingBlock: "12px",
        paddingInline: "28px",
        position: "relative",
    },
    nTop: {
        display: "flex",
        fontSize: "13px",
        justifyContent: "space-between",
    },
    nMuted: {
        color: marks.grey,
    },
    nTitle: {
        fontSize: "28px",
        fontWeight: 700,
        paddingTop: "18px",
    },
    nViews: {
        alignItems: "center",
        borderBottomColor: notion.sand,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "14px",
        paddingBottom: "6px",
    },
    nView: {
        color: marks.grey,
        fontSize: "13px",
    },
    nViewOn: {
        color: notion.ink,
        fontWeight: 600,
    },
    nTools: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
        marginLeft: "auto",
    },
    nBolt: {
        backgroundColor: notion.sky,
        borderRadius: "4px",
        color: notion.blue,
        fontSize: "12px",
        paddingBlock: "2px",
        paddingInline: "6px",
    },
    nNew: {
        backgroundColor: notion.link,
        borderRadius: "4px",
        color: marks.white,
        fontSize: "12px",
        fontWeight: 600,
        paddingBlock: "3px",
        paddingInline: "8px",
    },
    nBoard: {
        display: "grid",
        gap: "10px",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
    },
    nColumn: {
        alignContent: "start",
        display: "grid",
        gap: "6px",
    },
    nPill: {
        borderRadius: "4px",
        fontSize: "12px",
        justifySelf: "start",
        paddingBlock: "1px",
        paddingInline: "6px",
    },
    nCard: {
        backgroundColor: notion.surface,
        borderRadius: "6px",
        boxShadow: `0 0 0 1px ${notion.sand}, 0 1px 2px ${marks.shadow}`,
        display: "grid",
        fontSize: "13px",
        gap: "6px",
        paddingBlock: "8px",
        paddingInline: "10px",
    },
    nOwner: {
        alignItems: "center",
        backgroundColor: notion.sand,
        borderRadius: "50%",
        display: "inline-flex",
        fontSize: "10px",
        height: "18px",
        justifyContent: "center",
        width: "18px",
    },
    nPanel: {
        backgroundColor: notion.surface,
        borderRadius: "10px",
        boxShadow: `0 0 0 1px ${notion.sand}, 0 12px 32px ${marks.lift}`,
        display: "grid",
        gap: "8px",
        paddingBlock: "14px",
        paddingInline: "16px",
        position: "absolute",
        right: "20px",
        top: "96px",
        width: { default: "300px", [NARROW]: "calc(100% - 40px)" },
    },
    nPanelTitle: {
        fontSize: "15px",
    },
    nPanelHead: {
        color: marks.grey,
        fontSize: "12px",
        fontWeight: 600,
        paddingTop: "4px",
    },
    nRule: {
        alignItems: "center",
        backgroundColor: marks.column,
        borderRadius: "6px",
        display: "flex",
        flexWrap: "wrap",
        fontSize: "13px",
        gap: "6px",
        paddingBlock: "7px",
        paddingInline: "10px",
    },
    nPanelFoot: {
        alignItems: "center",
        display: "flex",
        fontSize: "12px",
        justifyContent: "space-between",
        paddingTop: "4px",
    },
    nSave: {
        backgroundColor: notion.link,
        borderRadius: "4px",
        color: marks.white,
        fontWeight: 600,
        paddingBlock: "4px",
        paddingInline: "12px",
    },

    // gmail
    gmail: {
        backgroundColor: gmail.canvas,
        color: gmail.ink,
        display: "grid",
        fontFamily: faces.gmail,
        fontSize: "14px",
        gridTemplateRows: "60px minmax(0, 1fr)",
        height: "100%",
    },
    gBar: {
        alignItems: "center",
        color: gmail.muted,
        display: "flex",
        gap: "16px",
        paddingInline: "18px",
    },
    gLogo: {
        alignItems: "center",
        color: gmail.muted,
        display: { default: "flex", [NARROW]: "none" },
        fontSize: "20px",
        gap: "6px",
        width: "150px",
    },
    gSearch: {
        alignItems: "center",
        backgroundColor: gmail.field,
        borderRadius: "999px",
        display: "flex",
        flexGrow: 1,
        gap: "12px",
        maxWidth: "520px",
        paddingBlock: "10px",
        paddingInline: "16px",
    },
    gPush: {
        flexGrow: 1,
    },
    gAvatar: {
        alignItems: "center",
        backgroundColor: marks.teal,
        borderRadius: "50%",
        color: marks.white,
        display: "inline-flex",
        fontSize: "13px",
        height: "30px",
        justifyContent: "center",
        marginLeft: "auto",
        width: "30px",
    },
    gBody: {
        display: "grid",
        gridTemplateColumns: { default: "190px minmax(0, 1fr)", [NARROW]: "minmax(0, 1fr)" },
        minHeight: 0,
        paddingRight: { default: "14px", [NARROW]: 0 },
    },
    gNav: {
        alignContent: "start",
        display: { default: "grid", [NARROW]: "none" },
        gap: "1px",
        paddingInline: "8px",
    },
    gCompose: {
        alignItems: "center",
        backgroundColor: gmail.compose,
        borderRadius: "16px",
        display: "flex",
        fontWeight: 500,
        gap: "12px",
        justifySelf: "start",
        marginBottom: "12px",
        paddingBlock: "16px",
        paddingInline: "18px",
    },
    gFolder: {
        alignItems: "center",
        borderRadius: "999px",
        display: "flex",
        fontSize: "13px",
        gap: "14px",
        paddingBlock: "6px",
        paddingInline: "14px",
    },
    gCount: {
        fontSize: "12px",
        fontWeight: 600,
        marginLeft: "auto",
    },
    gMain: {
        alignContent: "start",
        backgroundColor: gmail.surface,
        borderStartEndRadius: { default: "16px", [NARROW]: 0 },
        borderStartStartRadius: { default: "16px", [NARROW]: 0 },
        display: "grid",
        gap: "10px",
        overflow: "hidden",
        paddingBlock: "18px",
        paddingInline: "22px",
    },
    gTitle: {
        fontSize: "20px",
        fontWeight: 400,
    },
    gTabs: {
        borderBottomColor: gmail.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        fontSize: "13px",
        gap: "18px",
        overflow: "hidden",
        whiteSpace: "nowrap",
    },
    gTab: {
        color: gmail.muted,
        paddingBottom: "8px",
    },
    gTabOn: {
        borderBottomColor: gmail.blue,
        borderBottomStyle: "solid",
        borderBottomWidth: "3px",
        color: gmail.blue,
        fontWeight: 600,
    },
    gNote: {
        fontSize: "13px",
        paddingTop: "4px",
    },
    gFilter: {
        alignItems: "center",
        borderBottomColor: gmail.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        fontSize: "13px",
        gap: "14px",
        paddingBlock: "10px",
    },
    gCheck: {
        borderColor: gmail.muted,
        borderRadius: "2px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        flexShrink: 0,
        height: "12px",
        width: "12px",
    },
    gFilterText: {
        display: "grid",
        flexGrow: 1,
        gap: "2px",
    },
    gLinks: {
        color: gmail.blue,
        whiteSpace: "nowrap",
    },
    gNew: {
        color: gmail.blue,
        fontSize: "13px",
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
