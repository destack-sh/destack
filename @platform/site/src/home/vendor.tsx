import * as stylex from "@destack/style";
import type { JSX } from "@destack/view";

import {
    coverOf,
    joiners,
    launch,
    messages,
    month,
    perks,
    posts,
    sources,
    tasks,
    times,
    type TaskState,
} from "./corner";
import { Favicon, Glyph } from "./glyph";

/** The system font the rented sites fall back to. */
const system = '-apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif';

/** The typeface each rented site sets, or the nearest one a machine has. */
const faces = {
    linktree: '"Link Sans", "Avenir Next", Avenir, "Century Gothic", sans-serif',
    framer: '"Inter Display", Inter, "Helvetica Neue", Arial, sans-serif',
    calendly: `Gilroy, "Proxima Nova", ${system}`,
    notion: '"Lyon-Text", Georgia, ui-serif, "Times New Roman", serif',
    linear: `"Inter Variable", Inter, ${system}`,
    slack: `Lato, "Slack-Lato", ${system}`,
    github: `${system}, "Noto Sans"`,
};

/** Draw the rented site that holds a ledger row, at the size of a real window. */
export function rentedPageOf(row: number): JSX.Element {
    if (row === 1) {
        return <Notion />;
    } else if (row === 2) {
        return <Linear />;
    } else if (row === 3) {
        return <Slack />;
    } else if (row === 4) {
        return <GitHub />;
    } else if (row === 5) {
        return <Framer />;
    } else if (row === 6) {
        return <Calendly />;
    }

    return <Replit />;
}

/** Draw your waitlist as an app built by prompt and hosted on the builder's own domain. */
function Replit() {
    return (
        <div data-component="ReplitPage" {...stylex.attrs(styles.replit)}>
            <nav {...stylex.attrs(styles.replitNav)}>
                <b {...stylex.attrs(styles.replitBrand)}>
                    <span {...stylex.attrs(styles.replitLogo)}>✦</span>
                    LaunchKit
                </b>
                <span {...stylex.attrs(styles.replitLinks)}>
                    <span>Features</span>
                    <span>Pricing</span>
                    <span>FAQ</span>
                </span>
            </nav>
            <div {...stylex.attrs(styles.replitHero)}>
                <span {...stylex.attrs(styles.replitPill)}>🚀 Launching Friday</span>
                <b {...stylex.attrs(styles.replitTitle)}>
                    The launch tool{" "}
                    <span {...stylex.attrs(styles.replitGlow)}>you'll actually use</span>
                </b>
                <span {...stylex.attrs(styles.replitLine)}>
                    Pricing v4 opens on Friday. Leave your email and we'll send you an invite.
                </span>
                <span {...stylex.attrs(styles.replitForm)}>
                    <span {...stylex.attrs(styles.replitInput)}>you@company.com</span>
                    <span {...stylex.attrs(styles.replitButton)}>Join waitlist →</span>
                </span>
                <span {...stylex.attrs(styles.replitProof)}>
                    <span {...stylex.attrs(styles.replitFaces)}>
                        {joiners.map(([initial, tint]) => (
                            <span
                                style={{ "background-color": tint }}
                                {...stylex.attrs(styles.replitFace)}
                            >
                                {initial}
                            </span>
                        ))}
                    </span>
                    Join 1,840 others on the list
                </span>
            </div>
            <ul {...stylex.attrs(styles.replitPerks)}>
                {perks.map(([title, line]) => (
                    <li {...stylex.attrs(styles.replitPerk)}>
                        <b>{title}</b>
                        <span {...stylex.attrs(styles.replitPerkLine)}>{line}</span>
                    </li>
                ))}
            </ul>
            <span data-component="VendorBadge" {...stylex.attrs(styles.replitBadge)}>
                <Favicon icon="replit" tint="#f26207" size={14} />
                Made with Replit
            </span>
        </div>
    );
}

/** Draw your website as a builder's portfolio template: a big statement, selected work, and the people you have worked with. */
function Framer() {
    return (
        <div data-component="FramerPage" {...stylex.attrs(styles.framer)}>
            <nav {...stylex.attrs(styles.framerNav)}>
                <b {...stylex.attrs(styles.framerMark)}>florian</b>
                <span {...stylex.attrs(styles.framerLinks)}>
                    <span>Work</span>
                    <span>Writing</span>
                    <span>About</span>
                </span>
                <span {...stylex.attrs(styles.framerButton)}>Get in touch</span>
            </nav>
            <header {...stylex.attrs(styles.framerHero)}>
                <span {...stylex.attrs(styles.framerKicker)}>
                    <span {...stylex.attrs(styles.framerDot)} />
                    Available from November
                </span>
                <b {...stylex.attrs(styles.framerTitle)}>
                    I design and build small software for small teams.
                </b>
            </header>
            <section {...stylex.attrs(styles.framerSection)}>
                <span {...stylex.attrs(styles.framerHead)}>
                    <b>Selected work</b>
                    <span {...stylex.attrs(styles.framerMuted)}>2023 – 2025</span>
                </span>
                <ol {...stylex.attrs(styles.framerGrid)}>
                    {posts.map(([title, topic, date, from, to], index) => (
                        <li {...stylex.attrs(styles.framerCard)}>
                            <span
                                style={{ background: coverOf(index, from, to) }}
                                {...stylex.attrs(styles.framerCover)}
                            />
                            <span {...stylex.attrs(styles.framerMeta)}>
                                <b {...stylex.attrs(styles.framerPost)}>{title}</b>
                                <span {...stylex.attrs(styles.framerMuted)}>
                                    {topic} · {date.slice(-4)}
                                </span>
                            </span>
                        </li>
                    ))}
                </ol>
            </section>
            <span data-component="VendorBadge" {...stylex.attrs(styles.framerBadge)}>
                <Favicon icon="framer" tint="#000000" size={14} />
                Made in Framer
            </span>
        </div>
    );
}

/** Draw your booking page as a scheduling service's card, with a day chosen. */
function Calendly() {
    return (
        <div data-component="CalendlyPage" {...stylex.attrs(styles.calendly)}>
            <div {...stylex.attrs(styles.calCard)}>
                <div {...stylex.attrs(styles.calHost)}>
                    <span {...stylex.attrs(styles.calAvatar)}>F</span>
                    <b {...stylex.attrs(styles.calName)}>Florian</b>
                    <b {...stylex.attrs(styles.calEvent)}>30 Minute Meeting</b>
                    <span {...stylex.attrs(styles.calDetail)}>
                        <Glyph name="clock" size={20} />
                        30 min
                    </span>
                    <span {...stylex.attrs(styles.calDetail)}>
                        <Glyph name="video" size={20} />
                        Web conferencing details provided upon confirmation.
                    </span>
                </div>
                <div {...stylex.attrs(styles.calPicker)}>
                    <b {...stylex.attrs(styles.calTitle)}>Select a Date &amp; Time</b>
                    <span {...stylex.attrs(styles.calMonth)}>
                        <span {...stylex.attrs(styles.calStep, styles.calStepOff)}>
                            <Glyph name="back" size={18} />
                        </span>
                        October 2025
                        <span {...stylex.attrs(styles.calStep)}>
                            <Glyph name="forward" size={18} />
                        </span>
                    </span>
                    <div {...stylex.attrs(styles.calGrid)}>
                        {["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"].map((day) => (
                            <span {...stylex.attrs(styles.calWeekday)}>{day}</span>
                        ))}
                        {Array.from({ length: month.first }, () => (
                            <span />
                        ))}
                        {Array.from({ length: month.days }, (_, index) => index + 1).map((date) => (
                            <span
                                {...stylex.attrs(
                                    styles.calDate,
                                    month.open.includes(date) && styles.calOpen,
                                    date === month.chosen && styles.calChosen,
                                )}
                            >
                                {date}
                                {date === month.today ? (
                                    <span {...stylex.attrs(styles.calToday)} />
                                ) : undefined}
                            </span>
                        ))}
                    </div>
                    <span {...stylex.attrs(styles.calZone)}>
                        <b>Time zone</b>
                        <span {...stylex.attrs(styles.calZoneValue)}>
                            <Glyph name="globe" size={16} />
                            Central European Time (16:40)
                            <span {...stylex.attrs(styles.calCaret)} />
                        </span>
                    </span>
                </div>
                <div {...stylex.attrs(styles.calTimes)}>
                    <span {...stylex.attrs(styles.calDay)}>Tuesday, October 28</span>
                    {times.map((time) => (
                        <span {...stylex.attrs(styles.calTime)}>{time}</span>
                    ))}
                </div>
                <span data-component="VendorBadge" {...stylex.attrs(styles.calRibbon)}>
                    <span {...stylex.attrs(styles.calPowered)}>powered by</span>
                    Calendly
                </span>
            </div>
        </div>
    );
}

/** Draw the launch plan as a published page behind a public link. */
function Notion() {
    return (
        <div data-component="NotionPage" {...stylex.attrs(styles.notion)}>
            <nav {...stylex.attrs(styles.notionBar)}>
                <span {...stylex.attrs(styles.notionCrumb)}>
                    <span aria-hidden="true">🚀</span>
                    Launch plan
                </span>
                <span {...stylex.attrs(styles.notionTools)}>
                    <Glyph name="search" size={18} weight={1.75} />
                    <Glyph name="share" size={18} weight={1.75} />
                    <Glyph name="more" size={18} weight={2.5} />
                    <span {...stylex.attrs(styles.notionButton)}>Get Notion free</span>
                </span>
            </nav>
            <span {...stylex.attrs(styles.notionCover)} />
            <div {...stylex.attrs(styles.notionBody)}>
                <span aria-hidden="true" {...stylex.attrs(styles.notionIcon)}>
                    🚀
                </span>
                <b {...stylex.attrs(styles.notionTitle)}>Launch plan</b>
                <dl {...stylex.attrs(styles.notionProperties)}>
                    <dt {...stylex.attrs(styles.notionKey)}>
                        <Glyph name="status" size={16} weight={1.75} />
                        Status
                    </dt>
                    <dd {...stylex.attrs(styles.notionValue)}>
                        <span {...stylex.attrs(styles.notionStatus)}>
                            <span {...stylex.attrs(styles.notionDot)} />
                            In progress
                        </span>
                    </dd>
                    <dt {...stylex.attrs(styles.notionKey)}>
                        <Glyph name="calendar" size={16} weight={1.75} />
                        Launch
                    </dt>
                    <dd {...stylex.attrs(styles.notionValue)}>October 24, 2025</dd>
                    <dt {...stylex.attrs(styles.notionKey)}>
                        <Glyph name="person" size={16} weight={1.75} />
                        Owner
                    </dt>
                    <dd {...stylex.attrs(styles.notionValue)}>
                        <span {...stylex.attrs(styles.notionPerson)}>F</span>
                        Florian
                    </dd>
                </dl>
                <hr {...stylex.attrs(styles.notionRule)} />
                <p {...stylex.attrs(styles.notionCallout)}>
                    <span aria-hidden="true" {...stylex.attrs(styles.notionEmoji)}>
                        💡
                    </span>
                    Ship to the waitlist on Thursday, then open signups on Friday.
                </p>
                <b {...stylex.attrs(styles.notionHeading)}>Before launch</b>
                <ul {...stylex.attrs(styles.notionTodos)}>
                    {launch.map(([item, isDone]) => (
                        <li {...stylex.attrs(styles.notionTodo)}>
                            <span
                                {...stylex.attrs(styles.notionBox, isDone && styles.notionBoxDone)}
                            >
                                {isDone ? <Glyph name="check" size={12} weight={3} /> : undefined}
                            </span>
                            <span {...stylex.attrs(isDone && styles.notionDone)}>{item}</span>
                        </li>
                    ))}
                </ul>
            </div>
        </div>
    );
}

/** Draw the launch channel as a team chat seen through a guest invite. */
function Slack() {
    return (
        <div data-component="SlackPage" {...stylex.attrs(styles.slack)}>
            <div {...stylex.attrs(styles.slackTop)}>
                <span {...stylex.attrs(styles.slackSearch)}>
                    <Glyph name="search" size={14} />
                    Search LaunchKit
                </span>
            </div>
            <div {...stylex.attrs(styles.slackFrame)}>
                <nav {...stylex.attrs(styles.slackRail)}>
                    <span {...stylex.attrs(styles.slackTeamIcon)}>L</span>
                    <Glyph name="inbox" size={18} />
                    <Glyph name="mail" size={18} />
                    <Glyph name="grid" size={18} />
                </nav>
                <nav {...stylex.attrs(styles.slackSide)}>
                    <b {...stylex.attrs(styles.slackTeam)}>
                        LaunchKit <Glyph name="caret" size={12} />
                    </b>
                    <span {...stylex.attrs(styles.slackGroup)}>▾ Channels</span>
                    {["general", "launch", "design", "pricing", "random"].map((name) => (
                        <span
                            {...stylex.attrs(
                                styles.slackChannel,
                                name === "launch" && styles.slackOn,
                            )}
                        >
                            # {name}
                        </span>
                    ))}
                    <span {...stylex.attrs(styles.slackGroup)}>▾ Apps</span>
                    <span {...stylex.attrs(styles.slackChannel)}>
                        <span {...stylex.attrs(styles.slackBot)}>A</span>
                        Agent
                    </span>
                </nav>
                <div {...stylex.attrs(styles.slackMain)}>
                    <span {...stylex.attrs(styles.slackHeader)}>
                        <b># launch</b>
                        <span {...stylex.attrs(styles.slackMembers)}>
                            <span
                                {...stylex.attrs(styles.slackFace)}
                                style={{ "background-color": "#2f7d8c" }}
                            >
                                F
                            </span>
                            <span
                                {...stylex.attrs(styles.slackFace)}
                                style={{ "background-color": "#6b5ca5" }}
                            >
                                A
                            </span>
                            4
                        </span>
                    </span>
                    <span {...stylex.attrs(styles.slackTabs)}>
                        <span {...stylex.attrs(styles.slackTab, styles.slackTabOn)}>Messages</span>
                        <span {...stylex.attrs(styles.slackTab)}>Canvas</span>
                        <span {...stylex.attrs(styles.slackTab)}>Files</span>
                    </span>
                    <ul {...stylex.attrs(styles.slackMessages)}>
                        {messages.map(([author, time, text], index) => (
                            <li {...stylex.attrs(styles.slackMessage)}>
                                <span
                                    style={{
                                        "background-color":
                                            author === "Agent" ? "#6b5ca5" : "#2f7d8c",
                                    }}
                                    {...stylex.attrs(styles.slackAvatar)}
                                >
                                    {author === "Agent" ? "A" : "F"}
                                </span>
                                <span {...stylex.attrs(styles.slackBody)}>
                                    <span>
                                        <b>{author === "Agent" ? "Agent" : "Florian"}</b>
                                        {author === "Agent" ? (
                                            <span {...stylex.attrs(styles.slackApp)}>APP</span>
                                        ) : undefined}
                                        <span {...stylex.attrs(styles.slackMuted)}> {time}</span>
                                    </span>
                                    {text}
                                    {index === 1 ? (
                                        <span {...stylex.attrs(styles.slackReact)}>
                                            <span {...stylex.attrs(styles.slackChip)}>👍 2</span>
                                            <span {...stylex.attrs(styles.slackChip)}>🚀 1</span>
                                        </span>
                                    ) : undefined}
                                    {index === 2 ? (
                                        <span {...stylex.attrs(styles.slackThread)}>
                                            3 replies · Last reply today at 09:52
                                        </span>
                                    ) : undefined}
                                </span>
                            </li>
                        ))}
                    </ul>
                    <span {...stylex.attrs(styles.slackComposer)}>
                        <span {...stylex.attrs(styles.slackMuted)}>Message #launch</span>
                        <span {...stylex.attrs(styles.slackTools)}>
                            <span>+</span>
                            <span>Aa</span>
                            <span>☺</span>
                            <span>@</span>
                        </span>
                    </span>
                    <p {...stylex.attrs(styles.slackGuest)}>
                        You're a guest in LaunchKit. Ask an admin to see more channels.
                    </p>
                </div>
            </div>
        </div>
    );
}

/** Draw the source of your apps as a repository page on a code host. */
function GitHub() {
    return (
        <div data-component="GitHubPage" {...stylex.attrs(styles.github)}>
            <nav {...stylex.attrs(styles.ghBar)}>
                <Favicon icon="github" tint="#f0f6fc" size={30} />
                <span {...stylex.attrs(styles.ghPath)}>
                    florian / <b>launchkit</b>
                </span>
                <span {...stylex.attrs(styles.ghSearch)}>
                    <Glyph name="search" size={14} />
                    Type / to search
                </span>
            </nav>
            <span {...stylex.attrs(styles.ghTabs)}>
                {[
                    ["Code", ""],
                    ["Issues", "12"],
                    ["Pull requests", "3"],
                    ["Actions", ""],
                    ["Projects", ""],
                    ["Settings", ""],
                ].map(([tab, count], index) => (
                    <span {...stylex.attrs(styles.ghTab, index === 0 && styles.ghTabOn)}>
                        {tab}
                        {count === "" ? undefined : (
                            <span {...stylex.attrs(styles.ghCount)}>{count}</span>
                        )}
                    </span>
                ))}
            </span>
            <div {...stylex.attrs(styles.ghPage)}>
                <span {...stylex.attrs(styles.ghRepo)}>
                    <b {...stylex.attrs(styles.ghRepoName)}>launchkit</b>
                    <span {...stylex.attrs(styles.ghPill)}>Private</span>
                    <span {...stylex.attrs(styles.ghActions)}>
                        <span {...stylex.attrs(styles.ghButton)}>Watch 4</span>
                        <span {...stylex.attrs(styles.ghButton)}>Fork 0</span>
                        <span {...stylex.attrs(styles.ghButton)}>★ Star 3</span>
                    </span>
                </span>
                <div {...stylex.attrs(styles.ghColumns)}>
                    <div {...stylex.attrs(styles.ghBody)}>
                        <span {...stylex.attrs(styles.ghRow)}>
                            <span {...stylex.attrs(styles.ghBranch)}>⎇ main ▾</span>
                            <span {...stylex.attrs(styles.ghMuted)}>3 branches · 0 tags</span>
                            <span {...stylex.attrs(styles.ghCode)}>Code ▾</span>
                        </span>
                        <div {...stylex.attrs(styles.ghTable)}>
                            <span {...stylex.attrs(styles.ghCommit)}>
                                <span {...stylex.attrs(styles.ghFace)}>F</span>
                                <b>florian</b>
                                <span {...stylex.attrs(styles.ghMuted)}>
                                    Mail the first batch on Thursday
                                </span>
                                <span {...stylex.attrs(styles.ghMuted, styles.ghWhen)}>
                                    a41f2c9 · 2 hours ago · 128 commits
                                </span>
                            </span>
                            {sources.map(([name, change, when]) => (
                                <span {...stylex.attrs(styles.ghFile)}>
                                    <span {...stylex.attrs(styles.ghName)}>
                                        <Glyph
                                            name={name.includes(".") ? "file" : "folder"}
                                            size={16}
                                        />
                                        {name}
                                    </span>
                                    <span {...stylex.attrs(styles.ghMuted)}>{change}</span>
                                    <span {...stylex.attrs(styles.ghMuted, styles.ghWhen)}>
                                        {when}
                                    </span>
                                </span>
                            ))}
                        </div>
                    </div>
                    <aside {...stylex.attrs(styles.ghAbout)}>
                        <b>About</b>
                        <span {...stylex.attrs(styles.ghMuted)}>
                            The launch site, waitlist and weekly view for pricing v4.
                        </span>
                        <span {...stylex.attrs(styles.ghTopics)}>
                            {["launch", "waitlist", "typescript"].map((topic) => (
                                <span {...stylex.attrs(styles.ghTopic)}>{topic}</span>
                            ))}
                        </span>
                    </aside>
                </div>
            </div>
        </div>
    );
}

/** The groups of the issue list in the order a tracker shows them, with their names. */
const groups: readonly (readonly [state: TaskState, name: string])[] = [
    ["started", "In Progress"],
    ["review", "In Review"],
    ["todo", "Todo"],
    ["done", "Done"],
];

/** Draw the launch tasks as an issue tracker's list, seen from your own seat. */
function Linear() {
    return (
        <div data-component="LinearPage" {...stylex.attrs(styles.linear)}>
            <nav {...stylex.attrs(styles.linSide)}>
                <span {...stylex.attrs(styles.linSpace)}>
                    <span {...stylex.attrs(styles.linSpaceMark)}>L</span>
                    <b>Launch</b>
                    <Glyph name="caret" size={12} />
                    <span {...stylex.attrs(styles.linSpaceTools)}>
                        <Glyph name="search" size={15} weight={1.75} />
                        <span {...stylex.attrs(styles.linCompose)}>
                            <Glyph name="compose" size={14} weight={1.75} />
                        </span>
                    </span>
                </span>
                <span {...stylex.attrs(styles.linItem)}>
                    <Glyph name="inbox" size={15} weight={1.75} />
                    Inbox
                </span>
                <span {...stylex.attrs(styles.linItem)}>
                    <Glyph name="target" size={15} weight={1.75} />
                    My issues
                </span>
                <span {...stylex.attrs(styles.linGroup)}>Workspace</span>
                <span {...stylex.attrs(styles.linItem)}>
                    <Glyph name="layers" size={15} weight={1.75} />
                    Projects
                </span>
                <span {...stylex.attrs(styles.linItem)}>
                    <Glyph name="sliders" size={15} weight={1.75} />
                    Views
                </span>
                <span {...stylex.attrs(styles.linGroup)}>Your teams</span>
                <span {...stylex.attrs(styles.linItem)}>
                    <span {...stylex.attrs(styles.linTeam)}>L</span>
                    Launch
                </span>
                <span {...stylex.attrs(styles.linItem, styles.linSub, styles.linOn)}>
                    <Glyph name="target" size={15} weight={1.75} />
                    Issues
                </span>
                <span {...stylex.attrs(styles.linItem, styles.linSub)}>
                    <Glyph name="layers" size={15} weight={1.75} />
                    Projects
                </span>
                <span {...stylex.attrs(styles.linInvite)}>
                    <Glyph name="plus" size={14} weight={1.75} />
                    Invite people
                </span>
            </nav>
            <div {...stylex.attrs(styles.linMain)}>
                <span {...stylex.attrs(styles.linHeader)}>
                    <span {...stylex.attrs(styles.linTabs)}>
                        <span {...stylex.attrs(styles.linTab, styles.linTabOn)}>All issues</span>
                        <span {...stylex.attrs(styles.linTab)}>Active</span>
                        <span {...stylex.attrs(styles.linTab)}>Backlog</span>
                    </span>
                    <span {...stylex.attrs(styles.linTools)}>
                        <Glyph name="filter" size={16} weight={1.75} />
                        <span {...stylex.attrs(styles.linDisplay)}>
                            <Glyph name="sliders" size={14} weight={1.75} />
                            Display
                        </span>
                    </span>
                </span>
                {groups.map(([state, name]) => (
                    <>
                        <span {...stylex.attrs(styles.linGroupRow)}>
                            <StateMark state={state} />
                            <b {...stylex.attrs(styles.linGroupName)}>{name}</b>
                            <span {...stylex.attrs(styles.linCount)}>
                                {tasks.filter((task) => task[2] === state).length}
                            </span>
                        </span>
                        {tasks
                            .filter((task) => task[2] === state)
                            .map(([id, title, , owner, tint, due]) => (
                                <span {...stylex.attrs(styles.linRow)}>
                                    <span {...stylex.attrs(styles.linPriority)}>
                                        <span {...stylex.attrs(styles.linBar, styles.linBarLow)} />
                                        <span {...stylex.attrs(styles.linBar, styles.linBarMid)} />
                                        <span {...stylex.attrs(styles.linBar)} />
                                    </span>
                                    <span {...stylex.attrs(styles.linId)}>{id}</span>
                                    <StateMark state={state} />
                                    <span {...stylex.attrs(styles.linTitle)}>{title}</span>
                                    <span {...stylex.attrs(styles.linLabel)}>
                                        <span {...stylex.attrs(styles.linLabelDot)} />
                                        Launch
                                    </span>
                                    <span {...stylex.attrs(styles.linDue)}>{due}</span>
                                    <span
                                        style={{ "background-color": tint }}
                                        {...stylex.attrs(styles.linAvatar)}
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
        <svg aria-hidden="true" viewBox="0 0 14 14" {...stylex.attrs(styles.state)}>
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

/** The rented site styles, in the pixels of a real window. */
const styles = stylex.create({
    replit: {
        backgroundColor: "#0b0d17",
        backgroundImage:
            "radial-gradient(ellipse 60% 45% at 50% 0%, rgb(124 58 237 / 35%), transparent), radial-gradient(rgb(255 255 255 / 7%) 1px, transparent 1px)",
        backgroundSize: "auto, 22px 22px",
        color: "#f5f7fb",
        display: "grid",
        alignContent: "start",
        fontFamily: `Inter, ${system}`,
        gap: "28px",
        minHeight: "100%",
        padding: "0 48px 48px",
        position: "relative",
    },
    replitNav: {
        alignItems: "center",
        display: "flex",
        height: "72px",
        justifyContent: "space-between",
    },
    replitBrand: {
        alignItems: "center",
        display: "flex",
        fontSize: "18px",
        gap: "8px",
    },
    replitLogo: {
        color: "#a78bfa",
    },
    replitLinks: {
        color: "rgb(245 247 251 / 60%)",
        display: "flex",
        fontSize: "14px",
        gap: "28px",
    },
    replitHero: {
        display: "grid",
        gap: "14px",
        justifyItems: "center",
        margin: "0 auto",
        maxWidth: "640px",
        textAlign: "center",
    },
    replitPill: {
        backgroundColor: "rgb(255 255 255 / 6%)",
        borderColor: "rgb(255 255 255 / 14%)",
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        fontSize: "13px",
        padding: "6px 14px",
    },
    replitGlow: {
        backgroundClip: "text",
        backgroundImage: "linear-gradient(90deg, #a78bfa, #f472b6)",
        color: "transparent",
    },
    replitProof: {
        alignItems: "center",
        color: "rgb(245 247 251 / 60%)",
        display: "flex",
        fontSize: "14px",
        gap: "10px",
    },
    replitFaces: {
        display: "flex",
    },
    replitFace: {
        alignItems: "center",
        borderColor: "#0b0d17",
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "2px",
        color: "#ffffff",
        display: "flex",
        fontSize: "11px",
        fontWeight: 700,
        height: "28px",
        justifyContent: "center",
        marginLeft: "-8px",
        width: "28px",
    },
    replitPerks: {
        display: "grid",
        gap: "16px",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        listStyle: "none",
        margin: "0 auto",
        maxWidth: "820px",
        padding: 0,
        width: "100%",
    },
    replitPerk: {
        backgroundColor: "rgb(255 255 255 / 4%)",
        borderColor: "rgb(255 255 255 / 10%)",
        borderRadius: "14px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        fontSize: "15px",
        gap: "6px",
        padding: "18px",
    },
    replitPerkLine: {
        color: "rgb(245 247 251 / 55%)",
        fontSize: "13px",
    },
    replitTitle: {
        fontSize: "40px",
        fontWeight: 700,
        letterSpacing: "-0.03em",
        lineHeight: 1.1,
        maxWidth: "15ch",
    },
    replitLine: {
        color: "rgb(245 249 252 / 70%)",
        fontSize: "17px",
        lineHeight: 1.5,
    },
    replitForm: {
        display: "flex",
        gap: "10px",
        marginTop: "6px",
        maxWidth: "520px",
        width: "100%",
    },
    replitInput: {
        backgroundColor: "rgb(0 0 0 / 30%)",
        borderColor: "rgb(255 255 255 / 15%)",
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: "rgb(245 249 252 / 45%)",
        flexGrow: 1,
        fontSize: "16px",
        padding: "14px 16px",
        textAlign: "left",
    },
    replitButton: {
        backgroundImage: "linear-gradient(90deg, #7c3aed, #db2777)",
        borderRadius: "10px",
        fontSize: "16px",
        fontWeight: 600,
        padding: "14px 22px",
        whiteSpace: "nowrap",
    },
    replitBadge: {
        alignItems: "center",
        backgroundColor: "#0e1525",
        borderColor: "rgb(255 255 255 / 15%)",
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        bottom: "20px",
        display: "flex",
        fontSize: "13px",
        fontWeight: 600,
        gap: "6px",
        padding: "8px 12px",
        position: "absolute",
        right: "20px",
    },
    slack: {
        backgroundColor: "#ffffff",
        color: "#1d1c1d",
        display: "grid",
        fontFamily: faces.slack,
        fontSize: "15px",
        gridTemplateRows: "40px minmax(0, 1fr)",
        minHeight: "100%",
    },
    slackTop: {
        alignItems: "center",
        backgroundColor: "#350d36",
        display: "flex",
        justifyContent: "center",
    },
    slackSearch: {
        alignItems: "center",
        backgroundColor: "rgb(255 255 255 / 15%)",
        borderRadius: "6px",
        color: "rgb(255 255 255 / 80%)",
        display: "flex",
        fontSize: "13px",
        gap: "8px",
        paddingBlock: "5px",
        paddingInline: "12px",
        width: "48%",
    },
    slackFrame: {
        display: "grid",
        gridTemplateColumns: "64px 230px minmax(0, 1fr)",
        minHeight: 0,
    },
    slackRail: {
        alignContent: "start",
        backgroundColor: "#350d36",
        color: "rgb(255 255 255 / 70%)",
        display: "grid",
        gap: "22px",
        justifyItems: "center",
        paddingTop: "12px",
    },
    slackTeamIcon: {
        alignItems: "center",
        backgroundColor: "#ffffff",
        borderRadius: "8px",
        color: "#350d36",
        display: "flex",
        fontWeight: 800,
        height: "36px",
        justifyContent: "center",
        width: "36px",
    },
    slackSide: {
        alignContent: "start",
        backgroundColor: "#3f0e40",
        color: "rgb(255 255 255 / 72%)",
        display: "grid",
        gap: "1px",
        padding: "14px 10px",
    },
    slackTeam: {
        alignItems: "center",
        color: "#ffffff",
        display: "flex",
        fontSize: "18px",
        gap: "6px",
        marginBottom: "12px",
        paddingInline: "8px",
    },
    slackGroup: {
        fontSize: "14px",
        marginTop: "10px",
        paddingBlock: "4px",
        paddingInline: "8px",
    },
    slackChannel: {
        alignItems: "center",
        borderRadius: "6px",
        display: "flex",
        gap: "8px",
        paddingBlock: "4px",
        paddingInline: "14px",
    },
    slackOn: {
        backgroundColor: "#1164a3",
        color: "#ffffff",
        fontWeight: 700,
    },
    slackBot: {
        alignItems: "center",
        backgroundColor: "#6b5ca5",
        borderRadius: "4px",
        color: "#ffffff",
        display: "flex",
        fontSize: "11px",
        fontWeight: 700,
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    slackMain: {
        display: "grid",
        gridTemplateRows: "auto auto minmax(0, 1fr) auto auto",
        minWidth: 0,
    },
    slackHeader: {
        alignItems: "center",
        display: "flex",
        fontSize: "18px",
        justifyContent: "space-between",
        padding: "12px 20px 6px",
    },
    slackMembers: {
        alignItems: "center",
        borderColor: "#dddddd",
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        fontSize: "13px",
        gap: "4px",
        padding: "2px 8px 2px 4px",
    },
    slackFace: {
        alignItems: "center",
        borderRadius: "4px",
        color: "#ffffff",
        display: "flex",
        fontSize: "10px",
        fontWeight: 700,
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    slackTabs: {
        borderBottomColor: "#e8e8e8",
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "16px",
        paddingInline: "20px",
    },
    slackTab: {
        color: "#616061",
        fontSize: "13px",
        paddingBlock: "8px",
    },
    slackTabOn: {
        boxShadow: "inset 0 -2px 0 #1d1c1d",
        color: "#1d1c1d",
        fontWeight: 700,
    },
    slackMuted: {
        color: "#616061",
        fontSize: "13px",
    },
    slackMessages: {
        alignContent: "start",
        display: "grid",
        gap: "16px",
        listStyle: "none",
        margin: 0,
        overflow: "hidden",
        padding: "16px 20px",
    },
    slackMessage: {
        display: "flex",
        gap: "10px",
    },
    slackAvatar: {
        alignItems: "center",
        borderRadius: "6px",
        color: "#ffffff",
        display: "flex",
        flexShrink: 0,
        fontWeight: 700,
        height: "36px",
        justifyContent: "center",
        width: "36px",
    },
    slackBody: {
        display: "grid",
        gap: "3px",
        lineHeight: 1.45,
    },
    slackApp: {
        backgroundColor: "#e8e8e8",
        borderRadius: "3px",
        color: "#616061",
        fontSize: "10px",
        fontWeight: 700,
        marginLeft: "6px",
        paddingInline: "4px",
    },
    slackReact: {
        display: "flex",
        gap: "6px",
        marginTop: "2px",
    },
    slackChip: {
        backgroundColor: "#e8f5fa",
        borderColor: "#1d9bd1",
        borderRadius: "12px",
        borderStyle: "solid",
        borderWidth: "1px",
        fontSize: "12px",
        paddingInline: "8px",
    },
    slackThread: {
        color: "#1264a3",
        fontSize: "13px",
        fontWeight: 700,
    },
    slackComposer: {
        borderColor: "#bbbbbb",
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        gap: "10px",
        margin: "0 20px 10px",
        padding: "10px 12px",
    },
    slackTools: {
        color: "#616061",
        display: "flex",
        fontSize: "14px",
        gap: "16px",
    },
    slackGuest: {
        backgroundColor: "#fff8e1",
        color: "#5c4a00",
        fontSize: "13px",
        margin: 0,
        padding: "8px 20px",
    },
    github: {
        backgroundColor: "#0d1117",
        color: "#e6edf3",
        fontFamily: faces.github,
        fontSize: "14px",
        minHeight: "100%",
    },
    ghBar: {
        alignItems: "center",
        backgroundColor: "#010409",
        display: "flex",
        gap: "14px",
        height: "60px",
        paddingInline: "20px",
    },
    ghPath: {
        fontSize: "15px",
    },
    ghSearch: {
        alignItems: "center",
        borderColor: "#3d444d",
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: "#9198a1",
        display: "flex",
        fontSize: "13px",
        gap: "8px",
        marginLeft: "auto",
        padding: "5px 10px",
        width: "260px",
    },
    ghTabs: {
        backgroundColor: "#010409",
        borderBottomColor: "#3d444d",
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "2px",
        paddingInline: "16px",
    },
    ghTab: {
        alignItems: "center",
        color: "#e6edf3",
        display: "flex",
        gap: "6px",
        padding: "10px 12px",
    },
    ghTabOn: {
        boxShadow: "inset 0 -2px 0 #f78166",
        fontWeight: 600,
    },
    ghCount: {
        backgroundColor: "#2f3742",
        borderRadius: "999px",
        fontSize: "12px",
        paddingInline: "7px",
    },
    ghPage: {
        display: "grid",
        gap: "18px",
        padding: "20px 32px",
    },
    ghRepo: {
        alignItems: "center",
        borderBottomColor: "#3d444d",
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "10px",
        paddingBottom: "16px",
    },
    ghRepoName: {
        fontSize: "20px",
    },
    ghPill: {
        borderColor: "#3d444d",
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: "#9198a1",
        fontSize: "12px",
        paddingInline: "8px",
    },
    ghActions: {
        display: "flex",
        gap: "8px",
        marginLeft: "auto",
    },
    ghButton: {
        backgroundColor: "#212830",
        borderColor: "#3d444d",
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        fontSize: "12px",
        fontWeight: 600,
        padding: "3px 12px",
    },
    ghColumns: {
        display: "grid",
        gap: "24px",
        gridTemplateColumns: "minmax(0, 1fr) 240px",
    },
    ghBody: {
        alignContent: "start",
        display: "grid",
        gap: "14px",
        minWidth: 0,
    },
    ghRow: {
        alignItems: "center",
        display: "flex",
        gap: "14px",
    },
    ghBranch: {
        backgroundColor: "#212830",
        borderColor: "#3d444d",
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        padding: "5px 12px",
    },
    ghCode: {
        backgroundColor: "#238636",
        borderRadius: "6px",
        color: "#ffffff",
        fontWeight: 600,
        marginLeft: "auto",
        padding: "5px 14px",
    },
    ghTable: {
        borderColor: "#3d444d",
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        overflow: "hidden",
    },
    ghCommit: {
        alignItems: "center",
        backgroundColor: "#151b23",
        borderBottomColor: "#3d444d",
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "10px",
        height: "48px",
        paddingInline: "16px",
        whiteSpace: "nowrap",
    },
    ghFace: {
        alignItems: "center",
        backgroundColor: "#2f7d8c",
        borderRadius: "50%",
        display: "flex",
        fontSize: "11px",
        fontWeight: 700,
        height: "22px",
        justifyContent: "center",
        width: "22px",
    },
    ghFile: {
        alignItems: "center",
        borderBottomColor: "#3d444d",
        borderBottomStyle: "solid",
        borderBottomWidth: { default: "1px", ":last-child": 0 },
        display: "grid",
        gap: "16px",
        gridTemplateColumns: "minmax(0, 1fr) minmax(0, 1.2fr) 5.5rem",
        height: "40px",
        paddingInline: "16px",
        whiteSpace: "nowrap",
    },
    ghName: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
    },
    ghMuted: {
        color: "#9198a1",
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    ghWhen: {
        marginLeft: "auto",
        textAlign: "right",
    },
    ghAbout: {
        alignContent: "start",
        display: "grid",
        fontSize: "14px",
        gap: "10px",
        lineHeight: 1.5,
    },
    ghTopics: {
        display: "flex",
        flexWrap: "wrap",
        gap: "6px",
    },
    ghTopic: {
        backgroundColor: "#121d2f",
        borderRadius: "999px",
        color: "#4493f8",
        fontSize: "12px",
        fontWeight: 500,
        padding: "2px 10px",
    },
    framer: {
        backgroundColor: "#0a0a0a",
        color: "#ffffff",
        fontFamily: faces.framer,
        minHeight: "100%",
        position: "relative",
    },
    framerKicker: {
        alignItems: "center",
        color: "rgb(255 255 255 / 70%)",
        display: "flex",
        fontSize: "14px",
        gap: "8px",
    },
    framerDot: {
        backgroundColor: "#4ade80",
        borderRadius: "50%",
        boxShadow: "0 0 0 4px rgb(74 222 128 / 20%)",
        height: "8px",
        width: "8px",
    },
    framerMuted: {
        color: "rgb(255 255 255 / 50%)",
        fontSize: "14px",
    },
    framerNav: {
        alignItems: "center",
        display: "flex",
        gap: "32px",
        height: "80px",
        paddingInline: "40px",
    },
    framerMark: {
        fontSize: "22px",
        fontWeight: 700,
        letterSpacing: "-0.04em",
        marginRight: "auto",
    },
    framerLinks: {
        color: "rgb(255 255 255 / 60%)",
        display: "flex",
        fontSize: "15px",
        gap: "32px",
    },
    framerButton: {
        backgroundColor: "#ffffff",
        borderRadius: "999px",
        color: "#0a0a0a",
        fontSize: "14px",
        fontWeight: 600,
        paddingBlock: "10px",
        paddingInline: "18px",
    },
    framerHero: {
        display: "grid",
        gap: "20px",
        padding: "56px 40px 44px",
    },
    framerTitle: {
        fontSize: "56px",
        fontWeight: 600,
        letterSpacing: "-0.045em",
        lineHeight: 1.02,
        maxWidth: "16ch",
    },
    framerSection: {
        display: "grid",
        gap: "20px",
        paddingInline: "40px",
    },
    framerHead: {
        alignItems: "baseline",
        borderTopColor: "rgb(255 255 255 / 12%)",
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        display: "flex",
        fontSize: "15px",
        justifyContent: "space-between",
        paddingTop: "20px",
    },
    framerGrid: {
        display: "grid",
        gap: "16px",
        gridTemplateColumns: "1.4fr 1fr 1fr",
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    framerCard: {
        alignContent: "start",
        display: "grid",
        gap: "12px",
    },
    framerCover: {
        aspectRatio: "4 / 3",
        borderRadius: "14px",
    },
    framerMeta: {
        display: "grid",
        gap: "4px",
    },
    framerPost: {
        fontSize: "16px",
        fontWeight: 600,
        letterSpacing: "-0.01em",
    },
    framerBadge: {
        alignItems: "center",
        backgroundColor: "#ffffff",
        borderRadius: "10px",
        bottom: "20px",
        boxShadow: "0 4px 16px rgb(0 0 0 / 30%)",
        color: "#0a0a0a",
        display: "flex",
        fontSize: "13px",
        fontWeight: 600,
        gap: "6px",
        paddingBlock: "9px",
        paddingInline: "12px",
        position: "absolute",
        right: "20px",
    },
    calendly: {
        backgroundColor: "#fafafa",
        color: "#1a1a1a",
        display: "flex",
        fontFamily: faces.calendly,
        justifyContent: "center",
        minHeight: "100%",
        padding: "40px 32px",
    },
    calCard: {
        alignSelf: "start",
        backgroundColor: "#ffffff",
        borderColor: "rgb(26 26 26 / 10%)",
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        boxShadow: "0 1px 8px rgb(0 0 0 / 8%)",
        display: "grid",
        gridTemplateColumns: "220px minmax(0, 1fr) 168px",
        overflow: "hidden",
        position: "relative",
        width: "100%",
    },
    calHost: {
        alignContent: "start",
        borderRightColor: "rgb(26 26 26 / 10%)",
        borderRightStyle: "solid",
        borderRightWidth: "1px",
        display: "grid",
        padding: "28px 24px",
    },
    calAvatar: {
        alignItems: "center",
        backgroundColor: "#2f7d8c",
        borderRadius: "50%",
        color: "#ffffff",
        display: "flex",
        fontSize: "26px",
        fontWeight: 700,
        height: "64px",
        justifyContent: "center",
        marginBottom: "16px",
        width: "64px",
    },
    calName: {
        color: "rgb(26 26 26 / 61%)",
        fontSize: "16px",
        fontWeight: 700,
    },
    calEvent: {
        color: "#0a2540",
        fontSize: "28px",
        fontWeight: 700,
        lineHeight: 1.25,
        marginBottom: "24px",
        marginTop: "4px",
    },
    calDetail: {
        alignItems: "flex-start",
        color: "rgb(26 26 26 / 61%)",
        display: "flex",
        fontSize: "15px",
        fontWeight: 700,
        gap: "8px",
        lineHeight: 1.4,
        marginBottom: "16px",
    },
    calPicker: {
        alignContent: "start",
        display: "grid",
        padding: "28px 20px 28px 24px",
    },
    calTitle: {
        color: "#0a2540",
        fontSize: "20px",
        fontWeight: 700,
        marginBottom: "24px",
    },
    calMonth: {
        alignItems: "center",
        display: "flex",
        fontSize: "16px",
        gap: "24px",
        justifyContent: "center",
        marginBottom: "20px",
    },
    calStep: {
        alignItems: "center",
        backgroundColor: "rgb(0 105 255 / 6.5%)",
        borderRadius: "50%",
        color: "#0060e6",
        display: "flex",
        height: "36px",
        justifyContent: "center",
        width: "36px",
    },
    calStepOff: {
        backgroundColor: "transparent",
        color: "rgb(26 26 26 / 40%)",
    },
    calGrid: {
        display: "grid",
        gridTemplateColumns: "repeat(7, minmax(0, 1fr))",
        justifyItems: "center",
        rowGap: "6px",
    },
    calWeekday: {
        fontSize: "12px",
        paddingBottom: "4px",
        textAlign: "center",
    },
    calDate: {
        alignItems: "center",
        borderRadius: "50%",
        display: "flex",
        fontSize: "15px",
        height: "40px",
        justifyContent: "center",
        position: "relative",
        width: "40px",
    },
    calOpen: {
        backgroundColor: "rgb(0 105 255 / 6.5%)",
        color: "#0060e6",
        fontWeight: 700,
    },
    calChosen: {
        backgroundColor: "#0069ff",
        color: "#ffffff",
    },
    calToday: {
        backgroundColor: "currentColor",
        borderRadius: "50%",
        bottom: "6px",
        height: "4px",
        position: "absolute",
        width: "4px",
    },
    calZone: {
        display: "grid",
        fontSize: "15px",
        gap: "8px",
        marginTop: "28px",
    },
    calZoneValue: {
        alignItems: "center",
        display: "flex",
        fontSize: "14px",
        gap: "6px",
    },
    calCaret: {
        borderLeft: "4px solid transparent",
        borderRight: "4px solid transparent",
        borderTop: "5px solid currentColor",
        marginLeft: "4px",
    },
    calTimes: {
        alignContent: "start",
        display: "grid",
        gap: "10px",
        padding: "92px 20px 28px 0",
    },
    calDay: {
        fontSize: "16px",
        marginBottom: "6px",
    },
    calTime: {
        borderColor: "rgb(0 105 255 / 50%)",
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: "#0069ff",
        fontSize: "16px",
        fontWeight: 700,
        paddingBlock: "14px",
        textAlign: "center",
    },
    calRibbon: {
        backgroundColor: "#505960",
        color: "#ffffff",
        display: "grid",
        fontSize: "14px",
        fontWeight: 700,
        justifyItems: "center",
        lineHeight: 1.1,
        paddingBlock: "5px 6px",
        position: "absolute",
        right: "-44px",
        rotate: "45deg",
        top: "22px",
        width: "160px",
    },
    calPowered: {
        fontSize: "8px",
        letterSpacing: "0.04em",
        textTransform: "uppercase",
    },
    notion: {
        backgroundColor: "#ffffff",
        color: "#37352f",
        fontFamily: faces.notion,
        minHeight: "100%",
    },
    notionBar: {
        fontFamily: system,
        alignItems: "center",
        display: "flex",
        height: "45px",
        justifyContent: "space-between",
        paddingInline: "12px",
    },
    notionCrumb: {
        alignItems: "center",
        display: "flex",
        fontSize: "14px",
        gap: "6px",
        paddingInline: "6px",
    },
    notionTools: {
        alignItems: "center",
        color: "#37352f",
        display: "flex",
        gap: "16px",
    },
    notionButton: {
        backgroundColor: "#2f2f2f",
        borderRadius: "6px",
        color: "#ffffff",
        fontSize: "14px",
        fontWeight: 500,
        paddingBlock: "6px",
        paddingInline: "10px",
    },
    notionCover: {
        backgroundImage: "linear-gradient(120deg, #e8d5b5 0%, #d9a679 50%, #b46d4e 100%)",
        display: "block",
        height: "170px",
    },
    notionBody: {
        display: "grid",
        paddingInline: "72px",
    },
    notionIcon: {
        fontSize: "78px",
        lineHeight: 1,
        marginTop: "-40px",
    },
    notionTitle: {
        fontSize: "40px",
        fontWeight: 700,
        lineHeight: 1.2,
        marginBottom: "12px",
        marginTop: "16px",
    },
    notionProperties: {
        alignItems: "center",
        display: "grid",
        fontSize: "14px",
        gridAutoRows: "34px",
        gridTemplateColumns: "160px minmax(0, 1fr)",
        margin: 0,
    },
    notionKey: {
        alignItems: "center",
        color: "rgb(55 53 47 / 65%)",
        display: "flex",
        gap: "8px",
    },
    notionValue: {
        alignItems: "center",
        display: "flex",
        margin: 0,
    },
    notionStatus: {
        alignItems: "center",
        backgroundColor: "rgb(211 229 239)",
        borderRadius: "10px",
        color: "rgb(24 51 71)",
        display: "inline-flex",
        gap: "6px",
        paddingBlock: "1px",
        paddingInline: "8px 9px",
    },
    notionDot: {
        backgroundColor: "rgb(51 126 169)",
        borderRadius: "50%",
        height: "8px",
        width: "8px",
    },
    notionPerson: {
        alignItems: "center",
        backgroundColor: "#2f7d8c",
        borderRadius: "50%",
        color: "#ffffff",
        display: "flex",
        fontSize: "10px",
        fontWeight: 700,
        height: "20px",
        justifyContent: "center",
        marginRight: "6px",
        width: "20px",
    },
    notionRule: {
        borderColor: "rgb(55 53 47 / 9%)",
        borderStyle: "solid",
        borderWidth: "1px 0 0",
        margin: "16px 0 20px",
        width: "100%",
    },
    notionCallout: {
        alignItems: "flex-start",
        backgroundColor: "rgb(241 241 239)",
        borderRadius: "4px",
        display: "flex",
        fontSize: "16px",
        gap: "8px",
        lineHeight: 1.5,
        margin: 0,
        padding: "16px 16px 16px 12px",
    },
    notionEmoji: {
        fontSize: "21px",
        lineHeight: 1.1,
    },
    notionHeading: {
        fontSize: "24px",
        fontWeight: 600,
        marginBottom: "8px",
        marginTop: "32px",
    },
    notionTodos: {
        display: "grid",
        fontSize: "16px",
        gap: "6px",
        lineHeight: 1.5,
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    notionTodo: {
        alignItems: "center",
        display: "flex",
        gap: "8px",
    },
    notionBox: {
        alignItems: "center",
        borderColor: "#37352f",
        borderRadius: "3px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        color: "#ffffff",
        display: "flex",
        flexShrink: 0,
        height: "16px",
        justifyContent: "center",
        width: "16px",
    },
    notionBoxDone: {
        backgroundColor: "#2383e2",
        borderColor: "#2383e2",
    },
    notionDone: {
        opacity: 0.375,
        textDecorationLine: "line-through",
    },
    linear: {
        backgroundColor: "#ffffff",
        color: "#282a30",
        display: "grid",
        fontFamily: faces.linear,
        fontSize: "13px",
        gridTemplateColumns: "220px minmax(0, 1fr)",
        minHeight: "100%",
    },
    linSide: {
        alignContent: "start",
        backgroundColor: "#f7f7f8",
        borderRightColor: "#e9e9eb",
        borderRightStyle: "solid",
        borderRightWidth: "1px",
        display: "grid",
        gap: "1px",
        padding: "12px 10px",
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
        backgroundColor: "#d9622b",
        borderRadius: "5px",
        color: "#ffffff",
        display: "flex",
        fontSize: "11px",
        fontWeight: 700,
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    linSpaceTools: {
        alignItems: "center",
        color: "#6b6f76",
        display: "flex",
        gap: "10px",
        marginLeft: "auto",
    },
    linCompose: {
        alignItems: "center",
        backgroundColor: "#ffffff",
        borderRadius: "50%",
        boxShadow: "0 0 0 1px #e0e0e3",
        display: "flex",
        height: "26px",
        justifyContent: "center",
        width: "26px",
    },
    linItem: {
        alignItems: "center",
        borderRadius: "6px",
        color: "#3c4149",
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
        backgroundColor: "#e9e9ec",
        color: "#282a30",
    },
    linGroup: {
        color: "#6b6f76",
        fontSize: "12px",
        fontWeight: 500,
        marginTop: "14px",
        paddingBlock: "4px",
        paddingInline: "8px",
    },
    linTeam: {
        alignItems: "center",
        backgroundColor: "#fde4d6",
        borderRadius: "4px",
        color: "#d9622b",
        display: "flex",
        fontSize: "10px",
        fontWeight: 700,
        height: "16px",
        justifyContent: "center",
        width: "16px",
    },
    linInvite: {
        alignItems: "center",
        color: "#6b6f76",
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
        borderBottomColor: "#ededef",
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        height: "44px",
        justifyContent: "space-between",
        paddingInline: "16px 20px",
    },
    linTabs: {
        display: "flex",
        gap: "6px",
    },
    linTab: {
        borderColor: "#e3e3e6",
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: "#3c4149",
        fontSize: "12px",
        fontWeight: 500,
        paddingBlock: "3px",
        paddingInline: "10px",
    },
    linTabOn: {
        backgroundColor: "#f0f0f2",
        color: "#282a30",
    },
    linTools: {
        alignItems: "center",
        color: "#6b6f76",
        display: "flex",
        gap: "14px",
    },
    linDisplay: {
        alignItems: "center",
        borderColor: "#e3e3e6",
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: "#3c4149",
        display: "flex",
        fontSize: "12px",
        fontWeight: 500,
        gap: "6px",
        paddingBlock: "3px",
        paddingInline: "8px",
    },
    linGroupRow: {
        alignItems: "center",
        backgroundColor: "#f4f4f5",
        display: "flex",
        gap: "10px",
        height: "36px",
        paddingInline: "24px",
    },
    linGroupName: {
        fontWeight: 500,
    },
    linCount: {
        color: "#6b6f76",
    },
    linRow: {
        alignItems: "center",
        borderBottomColor: "#f0f0f1",
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "12px",
        height: "44px",
        paddingInline: "24px",
    },
    linPriority: {
        alignItems: "end",
        display: "flex",
        gap: "1.5px",
        height: "12px",
        width: "14px",
    },
    linBar: {
        backgroundColor: "#6b6f76",
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
        color: "#6b6f76",
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
        borderColor: "#e3e3e6",
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: "#3c4149",
        display: "flex",
        fontSize: "12px",
        gap: "6px",
        paddingBlock: "1px",
        paddingInline: "8px",
    },
    linLabelDot: {
        backgroundColor: "#d9622b",
        borderRadius: "50%",
        height: "8px",
        width: "8px",
    },
    linDue: {
        color: "#6b6f76",
        fontSize: "12px",
        width: "48px",
    },
    linAvatar: {
        alignItems: "center",
        borderRadius: "50%",
        color: "#ffffff",
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
