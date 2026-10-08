import * as style from "@destack/style";
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
import { Favicon, Glyph } from "../figure/glyph";
import { calendly, framer, github, linear, notion, replit, slack } from "./vendor.stylex";
import { palette } from "../../palette.stylex";

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
        <div data-component="ReplitPage" {...style.attrs(styles.replit)}>
            <nav {...style.attrs(styles.replitNav)}>
                <b {...style.attrs(styles.replitBrand)}>
                    <span {...style.attrs(styles.replitLogo)}>✦</span>
                    LaunchKit
                </b>
                <span {...style.attrs(styles.replitLinks)}>
                    <span>Features</span>
                    <span>Pricing</span>
                    <span>FAQ</span>
                </span>
            </nav>
            <div {...style.attrs(styles.replitHero)}>
                <span {...style.attrs(styles.replitPill)}>🚀 Launching Friday</span>
                <b {...style.attrs(styles.replitTitle)}>
                    The launch tool{" "}
                    <span {...style.attrs(styles.replitGlow)}>you'll actually use</span>
                </b>
                <span {...style.attrs(styles.replitLine)}>
                    Pricing v4 opens on Friday. Leave your email and we'll send you an invite.
                </span>
                <span {...style.attrs(styles.replitForm)}>
                    <span {...style.attrs(styles.replitInput)}>you@company.com</span>
                    <span {...style.attrs(styles.replitButton)}>Join waitlist →</span>
                </span>
                <span {...style.attrs(styles.replitProof)}>
                    <span {...style.attrs(styles.replitFaces)}>
                        {joiners.map(([initial, tint]) => (
                            <span
                                style={{ "background-color": tint }}
                                {...style.attrs(styles.replitFace)}
                            >
                                {initial}
                            </span>
                        ))}
                    </span>
                    Join 1,840 others on the list
                </span>
            </div>
            <ul {...style.attrs(styles.replitPerks)}>
                {perks.map(([title, line]) => (
                    <li {...style.attrs(styles.replitPerk)}>
                        <b>{title}</b>
                        <span {...style.attrs(styles.replitPerkLine)}>{line}</span>
                    </li>
                ))}
            </ul>
            <span data-component="VendorBadge" {...style.attrs(styles.replitBadge)}>
                <Favicon icon="replit" tint="#f26207" size={14} />
                Made with Replit
            </span>
        </div>
    );
}

/** Draw your website as a builder's portfolio template: a big statement, selected work, and the people you have worked with. */
function Framer() {
    return (
        <div data-component="FramerPage" {...style.attrs(styles.framer)}>
            <nav {...style.attrs(styles.framerNav)}>
                <b {...style.attrs(styles.framerMark)}>florian</b>
                <span {...style.attrs(styles.framerLinks)}>
                    <span>Work</span>
                    <span>Writing</span>
                    <span>About</span>
                </span>
                <span {...style.attrs(styles.framerButton)}>Get in touch</span>
            </nav>
            <header {...style.attrs(styles.framerHero)}>
                <span {...style.attrs(styles.framerKicker)}>
                    <span {...style.attrs(styles.framerDot)} />
                    Available from November
                </span>
                <b {...style.attrs(styles.framerTitle)}>
                    I design and build small software for small teams.
                </b>
            </header>
            <section {...style.attrs(styles.framerSection)}>
                <span {...style.attrs(styles.framerHead)}>
                    <b>Selected work</b>
                    <span {...style.attrs(styles.framerMuted)}>2023 – 2025</span>
                </span>
                <ol {...style.attrs(styles.framerGrid)}>
                    {posts.map(([title, topic, date, from, to], index) => (
                        <li {...style.attrs(styles.framerCard)}>
                            <span
                                style={{ background: coverOf(index, from, to) }}
                                {...style.attrs(styles.framerCover)}
                            />
                            <span {...style.attrs(styles.framerMeta)}>
                                <b {...style.attrs(styles.framerPost)}>{title}</b>
                                <span {...style.attrs(styles.framerMuted)}>
                                    {topic} · {date.slice(-4)}
                                </span>
                            </span>
                        </li>
                    ))}
                </ol>
            </section>
            <span data-component="VendorBadge" {...style.attrs(styles.framerBadge)}>
                <Favicon icon="framer" tint="#000000" size={14} />
                Made in Framer
            </span>
        </div>
    );
}

/** Draw your booking page as a scheduling service's card, with a day chosen. */
function Calendly() {
    return (
        <div data-component="CalendlyPage" {...style.attrs(styles.calendly)}>
            <div {...style.attrs(styles.calCard)}>
                <div {...style.attrs(styles.calHost)}>
                    <span {...style.attrs(styles.calAvatar)}>F</span>
                    <b {...style.attrs(styles.calName)}>Florian</b>
                    <b {...style.attrs(styles.calEvent)}>30 Minute Meeting</b>
                    <span {...style.attrs(styles.calDetail)}>
                        <Glyph name="clock" size={20} />
                        30 min
                    </span>
                    <span {...style.attrs(styles.calDetail)}>
                        <Glyph name="video" size={20} />
                        Web conferencing details provided upon confirmation.
                    </span>
                </div>
                <div {...style.attrs(styles.calPicker)}>
                    <b {...style.attrs(styles.calTitle)}>Select a Date &amp; Time</b>
                    <span {...style.attrs(styles.calMonth)}>
                        <span {...style.attrs(styles.calStep, styles.calStepOff)}>
                            <Glyph name="back" size={18} />
                        </span>
                        October 2025
                        <span {...style.attrs(styles.calStep)}>
                            <Glyph name="forward" size={18} />
                        </span>
                    </span>
                    <div {...style.attrs(styles.calGrid)}>
                        {["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"].map((day) => (
                            <span {...style.attrs(styles.calWeekday)}>{day}</span>
                        ))}
                        {Array.from({ length: month.first }, () => (
                            <span />
                        ))}
                        {Array.from({ length: month.days }, (_, index) => index + 1).map((date) => (
                            <span
                                {...style.attrs(
                                    styles.calDate,
                                    month.open.includes(date) && styles.calOpen,
                                    date === month.chosen && styles.calChosen,
                                )}
                            >
                                {date}
                                {date === month.today ? (
                                    <span {...style.attrs(styles.calToday)} />
                                ) : undefined}
                            </span>
                        ))}
                    </div>
                    <span {...style.attrs(styles.calZone)}>
                        <b>Time zone</b>
                        <span {...style.attrs(styles.calZoneValue)}>
                            <Glyph name="globe" size={16} />
                            Central European Time (16:40)
                            <span {...style.attrs(styles.calCaret)} />
                        </span>
                    </span>
                </div>
                <div {...style.attrs(styles.calTimes)}>
                    <span {...style.attrs(styles.calDay)}>Tuesday, October 28</span>
                    {times.map((time) => (
                        <span {...style.attrs(styles.calTime)}>{time}</span>
                    ))}
                </div>
                <span data-component="VendorBadge" {...style.attrs(styles.calRibbon)}>
                    <span {...style.attrs(styles.calPowered)}>powered by</span>
                    Calendly
                </span>
            </div>
        </div>
    );
}

/** Draw the launch plan as a published page behind a public link. */
function Notion() {
    return (
        <div data-component="NotionPage" {...style.attrs(styles.notion)}>
            <nav {...style.attrs(styles.notionBar)}>
                <span {...style.attrs(styles.notionCrumb)}>
                    <span aria-hidden="true">🚀</span>
                    Launch plan
                </span>
                <span {...style.attrs(styles.notionTools)}>
                    <Glyph name="search" size={18} weight={1.75} />
                    <Glyph name="share" size={18} weight={1.75} />
                    <Glyph name="more" size={18} weight={2.5} />
                    <span {...style.attrs(styles.notionButton)}>Get Notion free</span>
                </span>
            </nav>
            <span {...style.attrs(styles.notionCover)} />
            <div {...style.attrs(styles.notionBody)}>
                <span aria-hidden="true" {...style.attrs(styles.notionIcon)}>
                    🚀
                </span>
                <b {...style.attrs(styles.notionTitle)}>Launch plan</b>
                <dl {...style.attrs(styles.notionProperties)}>
                    <dt {...style.attrs(styles.notionKey)}>
                        <Glyph name="status" size={16} weight={1.75} />
                        Status
                    </dt>
                    <dd {...style.attrs(styles.notionValue)}>
                        <span {...style.attrs(styles.notionStatus)}>
                            <span {...style.attrs(styles.notionDot)} />
                            In progress
                        </span>
                    </dd>
                    <dt {...style.attrs(styles.notionKey)}>
                        <Glyph name="calendar" size={16} weight={1.75} />
                        Launch
                    </dt>
                    <dd {...style.attrs(styles.notionValue)}>October 24, 2025</dd>
                    <dt {...style.attrs(styles.notionKey)}>
                        <Glyph name="person" size={16} weight={1.75} />
                        Owner
                    </dt>
                    <dd {...style.attrs(styles.notionValue)}>
                        <span {...style.attrs(styles.notionPerson)}>F</span>
                        Florian
                    </dd>
                </dl>
                <hr {...style.attrs(styles.notionRule)} />
                <p {...style.attrs(styles.notionCallout)}>
                    <span aria-hidden="true" {...style.attrs(styles.notionEmoji)}>
                        💡
                    </span>
                    Ship to the waitlist on Thursday, then open signups on Friday.
                </p>
                <b {...style.attrs(styles.notionHeading)}>Before launch</b>
                <ul {...style.attrs(styles.notionTodos)}>
                    {launch.map(([item, isDone]) => (
                        <li {...style.attrs(styles.notionTodo)}>
                            <span
                                {...style.attrs(styles.notionBox, isDone && styles.notionBoxDone)}
                            >
                                {isDone ? <Glyph name="check" size={12} weight={3} /> : undefined}
                            </span>
                            <span {...style.attrs(isDone && styles.notionDone)}>{item}</span>
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
        <div data-component="SlackPage" {...style.attrs(styles.slack)}>
            <div {...style.attrs(styles.slackTop)}>
                <span {...style.attrs(styles.slackSearch)}>
                    <Glyph name="search" size={14} />
                    Search LaunchKit
                </span>
            </div>
            <div {...style.attrs(styles.slackFrame)}>
                <nav {...style.attrs(styles.slackRail)}>
                    <span {...style.attrs(styles.slackTeamIcon)}>L</span>
                    <Glyph name="inbox" size={18} />
                    <Glyph name="mail" size={18} />
                    <Glyph name="grid" size={18} />
                </nav>
                <nav {...style.attrs(styles.slackSide)}>
                    <b {...style.attrs(styles.slackTeam)}>
                        LaunchKit <Glyph name="caret" size={12} />
                    </b>
                    <span {...style.attrs(styles.slackGroup)}>▾ Channels</span>
                    {["general", "launch", "design", "pricing", "random"].map((name) => (
                        <span
                            {...style.attrs(
                                styles.slackChannel,
                                name === "launch" && styles.slackOn,
                            )}
                        >
                            # {name}
                        </span>
                    ))}
                    <span {...style.attrs(styles.slackGroup)}>▾ Apps</span>
                    <span {...style.attrs(styles.slackChannel)}>
                        <span {...style.attrs(styles.slackBot)}>A</span>
                        Agent
                    </span>
                </nav>
                <div {...style.attrs(styles.slackMain)}>
                    <span {...style.attrs(styles.slackHeader)}>
                        <b># launch</b>
                        <span {...style.attrs(styles.slackMembers)}>
                            <span
                                {...style.attrs(styles.slackFace)}
                                style={{ "background-color": "#2f7d8c" }}
                            >
                                F
                            </span>
                            <span
                                {...style.attrs(styles.slackFace)}
                                style={{ "background-color": "#6b5ca5" }}
                            >
                                A
                            </span>
                            4
                        </span>
                    </span>
                    <span {...style.attrs(styles.slackTabs)}>
                        <span {...style.attrs(styles.slackTab, styles.slackTabOn)}>Messages</span>
                        <span {...style.attrs(styles.slackTab)}>Canvas</span>
                        <span {...style.attrs(styles.slackTab)}>Files</span>
                    </span>
                    <ul {...style.attrs(styles.slackMessages)}>
                        {messages.map(([author, time, text], index) => (
                            <li {...style.attrs(styles.slackMessage)}>
                                <span
                                    style={{
                                        "background-color":
                                            author === "Agent" ? "#6b5ca5" : "#2f7d8c",
                                    }}
                                    {...style.attrs(styles.slackAvatar)}
                                >
                                    {author === "Agent" ? "A" : "F"}
                                </span>
                                <span {...style.attrs(styles.slackBody)}>
                                    <span>
                                        <b>{author === "Agent" ? "Agent" : "Florian"}</b>
                                        {author === "Agent" ? (
                                            <span {...style.attrs(styles.slackApp)}>APP</span>
                                        ) : undefined}
                                        <span {...style.attrs(styles.slackMuted)}> {time}</span>
                                    </span>
                                    {text}
                                    {index === 1 ? (
                                        <span {...style.attrs(styles.slackReact)}>
                                            <span {...style.attrs(styles.slackChip)}>👍 2</span>
                                            <span {...style.attrs(styles.slackChip)}>🚀 1</span>
                                        </span>
                                    ) : undefined}
                                    {index === 2 ? (
                                        <span {...style.attrs(styles.slackThread)}>
                                            3 replies · Last reply today at 09:52
                                        </span>
                                    ) : undefined}
                                </span>
                            </li>
                        ))}
                    </ul>
                    <span {...style.attrs(styles.slackComposer)}>
                        <span {...style.attrs(styles.slackMuted)}>Message #launch</span>
                        <span {...style.attrs(styles.slackTools)}>
                            <span>+</span>
                            <span>Aa</span>
                            <span>☺</span>
                            <span>@</span>
                        </span>
                    </span>
                    <p {...style.attrs(styles.slackGuest)}>
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
        <div data-component="GitHubPage" {...style.attrs(styles.github)}>
            <nav {...style.attrs(styles.ghBar)}>
                <Favicon icon="github" tint="#f0f6fc" size={30} />
                <span {...style.attrs(styles.ghPath)}>
                    florian / <b>launchkit</b>
                </span>
                <span {...style.attrs(styles.ghSearch)}>
                    <Glyph name="search" size={14} />
                    Type / to search
                </span>
            </nav>
            <span {...style.attrs(styles.ghTabs)}>
                {[
                    ["Code", ""],
                    ["Issues", "12"],
                    ["Pull requests", "3"],
                    ["Actions", ""],
                    ["Projects", ""],
                    ["Settings", ""],
                ].map(([tab, count], index) => (
                    <span {...style.attrs(styles.ghTab, index === 0 && styles.ghTabOn)}>
                        {tab}
                        {count === "" ? undefined : (
                            <span {...style.attrs(styles.ghCount)}>{count}</span>
                        )}
                    </span>
                ))}
            </span>
            <div {...style.attrs(styles.ghPage)}>
                <span {...style.attrs(styles.ghRepo)}>
                    <b {...style.attrs(styles.ghRepoName)}>launchkit</b>
                    <span {...style.attrs(styles.ghPill)}>Private</span>
                    <span {...style.attrs(styles.ghActions)}>
                        <span {...style.attrs(styles.ghButton)}>Watch 4</span>
                        <span {...style.attrs(styles.ghButton)}>Fork 0</span>
                        <span {...style.attrs(styles.ghButton)}>★ Star 3</span>
                    </span>
                </span>
                <div {...style.attrs(styles.ghColumns)}>
                    <div {...style.attrs(styles.ghBody)}>
                        <span {...style.attrs(styles.ghRow)}>
                            <span {...style.attrs(styles.ghBranch)}>⎇ main ▾</span>
                            <span {...style.attrs(styles.ghMuted)}>3 branches · 0 tags</span>
                            <span {...style.attrs(styles.ghCode)}>Code ▾</span>
                        </span>
                        <div {...style.attrs(styles.ghTable)}>
                            <span {...style.attrs(styles.ghCommit)}>
                                <span {...style.attrs(styles.ghFace)}>F</span>
                                <b>florian</b>
                                <span {...style.attrs(styles.ghMuted)}>
                                    Mail the first batch on Thursday
                                </span>
                                <span {...style.attrs(styles.ghMuted, styles.ghWhen)}>
                                    a41f2c9 · 2 hours ago · 128 commits
                                </span>
                            </span>
                            {sources.map(([name, change, when]) => (
                                <span {...style.attrs(styles.ghFile)}>
                                    <span {...style.attrs(styles.ghName)}>
                                        <Glyph
                                            name={name.includes(".") ? "file" : "folder"}
                                            size={16}
                                        />
                                        {name}
                                    </span>
                                    <span {...style.attrs(styles.ghMuted)}>{change}</span>
                                    <span {...style.attrs(styles.ghMuted, styles.ghWhen)}>
                                        {when}
                                    </span>
                                </span>
                            ))}
                        </div>
                    </div>
                    <aside {...style.attrs(styles.ghAbout)}>
                        <b>About</b>
                        <span {...style.attrs(styles.ghMuted)}>
                            The launch site, waitlist and weekly view for pricing v4.
                        </span>
                        <span {...style.attrs(styles.ghTopics)}>
                            {["launch", "waitlist", "typescript"].map((topic) => (
                                <span {...style.attrs(styles.ghTopic)}>{topic}</span>
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
        <div data-component="LinearPage" {...style.attrs(styles.linear)}>
            <nav {...style.attrs(styles.linSide)}>
                <span {...style.attrs(styles.linSpace)}>
                    <span {...style.attrs(styles.linSpaceMark)}>L</span>
                    <b>Launch</b>
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
                    <span {...style.attrs(styles.linTeam)}>L</span>
                    Launch
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

/** The rented site styles, in the pixels of a real window. */
const styles = style.create({
    replit: {
        backgroundColor: replit.night,
        backgroundImage: `radial-gradient(ellipse 60% 45% at 50% 0%, color-mix(in srgb, ${replit.violet} 35%, transparent), transparent), radial-gradient(color-mix(in srgb, white 7%, transparent) 1px, transparent 1px)`,
        backgroundSize: "auto, 22px 22px",
        color: replit.snow,
        display: "grid",
        alignContent: "start",
        fontFamily: `Inter, ${system}`,
        gap: "28px",
        minHeight: "100%",
        paddingBlockStart: "0",
        paddingBlockEnd: "48px",
        paddingInline: "48px",
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
        color: replit.lilac,
    },
    replitLinks: {
        color: `color-mix(in srgb, ${replit.snow} 60%, transparent)`,
        display: "flex",
        fontSize: "14px",
        gap: "28px",
    },
    replitHero: {
        display: "grid",
        gap: "14px",
        justifyItems: "center",
        marginBlock: "0",
        marginInline: "auto",
        maxWidth: "640px",
        textAlign: "center",
    },
    replitPill: {
        backgroundColor: `color-mix(in srgb, white 6%, transparent)`,
        borderColor: `color-mix(in srgb, white 14%, transparent)`,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        fontSize: "13px",
        paddingBlock: "6px",
        paddingInline: "14px",
    },
    replitGlow: {
        backgroundClip: "text",
        backgroundImage: `linear-gradient(90deg, ${replit.lilac}, ${replit.pink})`,
        color: "transparent",
    },
    replitProof: {
        alignItems: "center",
        color: `color-mix(in srgb, ${replit.snow} 60%, transparent)`,
        display: "flex",
        fontSize: "14px",
        gap: "10px",
    },
    replitFaces: {
        display: "flex",
    },
    replitFace: {
        alignItems: "center",
        borderColor: replit.night,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "2px",
        color: "white",
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
        marginBlock: "0",
        marginInline: "auto",
        maxWidth: "820px",
        padding: 0,
        width: "100%",
    },
    replitPerk: {
        backgroundColor: `color-mix(in srgb, white 4%, transparent)`,
        borderColor: `color-mix(in srgb, white 10%, transparent)`,
        borderRadius: "14px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        fontSize: "15px",
        gap: "6px",
        padding: "18px",
    },
    replitPerkLine: {
        color: `color-mix(in srgb, ${replit.snow} 55%, transparent)`,
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
        color: `color-mix(in srgb, ${replit.snow} 70%, transparent)`,
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
        backgroundColor: "color-mix(in srgb, black 30%, transparent)",
        borderColor: `color-mix(in srgb, white 15%, transparent)`,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: `color-mix(in srgb, ${replit.snow} 45%, transparent)`,
        flexGrow: 1,
        fontSize: "16px",
        paddingBlock: "14px",
        paddingInline: "16px",
        textAlign: "left",
    },
    replitButton: {
        backgroundImage: `linear-gradient(90deg, ${replit.violet}, ${replit.magenta})`,
        borderRadius: "10px",
        fontSize: "16px",
        fontWeight: 600,
        paddingBlock: "14px",
        paddingInline: "22px",
        whiteSpace: "nowrap",
    },
    replitBadge: {
        alignItems: "center",
        backgroundColor: replit.navy,
        borderColor: `color-mix(in srgb, white 15%, transparent)`,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        bottom: "20px",
        display: "flex",
        fontSize: "13px",
        fontWeight: 600,
        gap: "6px",
        paddingBlock: "8px",
        paddingInline: "12px",
        position: "absolute",
        right: "20px",
    },
    slack: {
        backgroundColor: "white",
        color: slack.ink,
        display: "grid",
        fontFamily: faces.slack,
        fontSize: "15px",
        gridTemplateRows: "40px minmax(0, 1fr)",
        minHeight: "100%",
    },
    slackTop: {
        alignItems: "center",
        backgroundColor: slack.aubergine,
        display: "flex",
        justifyContent: "center",
    },
    slackSearch: {
        alignItems: "center",
        backgroundColor: `color-mix(in srgb, white 15%, transparent)`,
        borderRadius: "6px",
        color: `color-mix(in srgb, white 80%, transparent)`,
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
        backgroundColor: slack.aubergine,
        color: `color-mix(in srgb, white 70%, transparent)`,
        display: "grid",
        gap: "22px",
        justifyItems: "center",
        paddingTop: "12px",
    },
    slackTeamIcon: {
        alignItems: "center",
        backgroundColor: "white",
        borderRadius: "8px",
        color: slack.aubergine,
        display: "flex",
        fontWeight: 800,
        height: "36px",
        justifyContent: "center",
        width: "36px",
    },
    slackSide: {
        alignContent: "start",
        backgroundColor: slack.sidebar,
        color: `color-mix(in srgb, white 72%, transparent)`,
        display: "grid",
        gap: "1px",
        paddingBlock: "14px",
        paddingInline: "10px",
    },
    slackTeam: {
        alignItems: "center",
        color: "white",
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
        backgroundColor: slack.link,
        color: "white",
        fontWeight: 700,
    },
    slackBot: {
        alignItems: "center",
        backgroundColor: palette.violet,
        borderRadius: "4px",
        color: "white",
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
        paddingBlockStart: "12px",
        paddingBlockEnd: "6px",
        paddingInline: "20px",
    },
    slackMembers: {
        alignItems: "center",
        borderColor: slack.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        fontSize: "13px",
        gap: "4px",
        paddingBlock: "2px",
        paddingInlineStart: "4px",
        paddingInlineEnd: "8px",
    },
    slackFace: {
        alignItems: "center",
        borderRadius: "4px",
        color: "white",
        display: "flex",
        fontSize: "10px",
        fontWeight: 700,
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    slackTabs: {
        borderBottomColor: slack.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "16px",
        paddingInline: "20px",
    },
    slackTab: {
        color: slack.muted,
        fontSize: "13px",
        paddingBlock: "8px",
    },
    slackTabOn: {
        boxShadow: `inset 0 -2px 0 ${slack.ink}`,
        color: slack.ink,
        fontWeight: 700,
    },
    slackMuted: {
        color: slack.muted,
        fontSize: "13px",
    },
    slackMessages: {
        alignContent: "start",
        display: "grid",
        gap: "16px",
        listStyle: "none",
        margin: 0,
        overflow: "hidden",
        paddingBlock: "16px",
        paddingInline: "20px",
    },
    slackMessage: {
        display: "flex",
        gap: "10px",
    },
    slackAvatar: {
        alignItems: "center",
        borderRadius: "6px",
        color: "white",
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
        backgroundColor: slack.rule,
        borderRadius: "3px",
        color: slack.muted,
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
        backgroundColor: slack.mention,
        borderColor: slack.highlight,
        borderRadius: "12px",
        borderStyle: "solid",
        borderWidth: "1px",
        fontSize: "12px",
        paddingInline: "8px",
    },
    slackThread: {
        color: slack.link,
        fontSize: "13px",
        fontWeight: 700,
    },
    slackComposer: {
        borderColor: slack.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        gap: "10px",
        marginBlockStart: "0",
        marginBlockEnd: "10px",
        marginInline: "20px",
        paddingBlock: "10px",
        paddingInline: "12px",
    },
    slackTools: {
        color: slack.muted,
        display: "flex",
        fontSize: "14px",
        gap: "16px",
    },
    slackGuest: {
        backgroundColor: slack.notice,
        color: slack.noticeInk,
        fontSize: "13px",
        margin: 0,
        paddingBlock: "8px",
        paddingInline: "20px",
    },
    github: {
        backgroundColor: github.canvas,
        color: github.foreground,
        fontFamily: faces.github,
        fontSize: "14px",
        minHeight: "100%",
    },
    ghBar: {
        alignItems: "center",
        backgroundColor: github.inset,
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
        borderColor: github.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: github.muted,
        display: "flex",
        fontSize: "13px",
        gap: "8px",
        marginLeft: "auto",
        paddingBlock: "5px",
        paddingInline: "10px",
        width: "260px",
    },
    ghTabs: {
        backgroundColor: github.inset,
        borderBottomColor: github.border,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "2px",
        paddingInline: "16px",
    },
    ghTab: {
        alignItems: "center",
        color: github.foreground,
        display: "flex",
        gap: "6px",
        paddingBlock: "10px",
        paddingInline: "12px",
    },
    ghTabOn: {
        boxShadow: `inset 0 -2px 0 ${github.attention}`,
        fontWeight: 600,
    },
    ghCount: {
        backgroundColor: github.neutral,
        borderRadius: "999px",
        fontSize: "12px",
        paddingInline: "7px",
    },
    ghPage: {
        display: "grid",
        gap: "18px",
        paddingBlock: "20px",
        paddingInline: "32px",
    },
    ghRepo: {
        alignItems: "center",
        borderBottomColor: github.border,
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
        borderColor: github.border,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: github.muted,
        fontSize: "12px",
        paddingInline: "8px",
    },
    ghActions: {
        display: "flex",
        gap: "8px",
        marginLeft: "auto",
    },
    ghButton: {
        backgroundColor: github.subtle,
        borderColor: github.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        fontSize: "12px",
        fontWeight: 600,
        paddingBlock: "3px",
        paddingInline: "12px",
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
        backgroundColor: github.subtle,
        borderColor: github.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        paddingBlock: "5px",
        paddingInline: "12px",
    },
    ghCode: {
        backgroundColor: github.success,
        borderRadius: "6px",
        color: "white",
        fontWeight: 600,
        marginLeft: "auto",
        paddingBlock: "5px",
        paddingInline: "14px",
    },
    ghTable: {
        borderColor: github.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        overflow: "hidden",
    },
    ghCommit: {
        alignItems: "center",
        backgroundColor: github.overlay,
        borderBottomColor: github.border,
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
        backgroundColor: palette.teal,
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
        borderBottomColor: github.border,
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
        color: github.muted,
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
        backgroundColor: github.accentMuted,
        borderRadius: "999px",
        color: github.accent,
        fontSize: "12px",
        fontWeight: 500,
        paddingBlock: "2px",
        paddingInline: "10px",
    },
    framer: {
        backgroundColor: framer.ink,
        color: "white",
        fontFamily: faces.framer,
        minHeight: "100%",
        position: "relative",
    },
    framerKicker: {
        alignItems: "center",
        color: `color-mix(in srgb, white 70%, transparent)`,
        display: "flex",
        fontSize: "14px",
        gap: "8px",
    },
    framerDot: {
        backgroundColor: framer.green,
        borderRadius: "50%",
        boxShadow: `0 0 0 4px color-mix(in srgb, ${framer.green} 20%, transparent)`,
        height: "8px",
        width: "8px",
    },
    framerMuted: {
        color: `color-mix(in srgb, white 50%, transparent)`,
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
        color: `color-mix(in srgb, white 60%, transparent)`,
        display: "flex",
        fontSize: "15px",
        gap: "32px",
    },
    framerButton: {
        backgroundColor: "white",
        borderRadius: "999px",
        color: framer.ink,
        fontSize: "14px",
        fontWeight: 600,
        paddingBlock: "10px",
        paddingInline: "18px",
    },
    framerHero: {
        display: "grid",
        gap: "20px",
        paddingBlockStart: "56px",
        paddingBlockEnd: "44px",
        paddingInline: "40px",
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
        borderTopColor: `color-mix(in srgb, white 12%, transparent)`,
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
        backgroundColor: "white",
        borderRadius: "10px",
        bottom: "20px",
        boxShadow: "0 4px 16px color-mix(in srgb, black 30%, transparent)",
        color: framer.ink,
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
        backgroundColor: calendly.mist,
        color: calendly.ink,
        display: "flex",
        fontFamily: faces.calendly,
        justifyContent: "center",
        minHeight: "100%",
        paddingBlock: "40px",
        paddingInline: "32px",
    },
    calCard: {
        alignSelf: "start",
        backgroundColor: "white",
        borderColor: `color-mix(in srgb, ${calendly.ink} 10%, transparent)`,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        boxShadow: "0 1px 8px color-mix(in srgb, black 8%, transparent)",
        display: "grid",
        gridTemplateColumns: "220px minmax(0, 1fr) 168px",
        overflow: "hidden",
        position: "relative",
        width: "100%",
    },
    calHost: {
        alignContent: "start",
        borderRightColor: `color-mix(in srgb, ${calendly.ink} 10%, transparent)`,
        borderRightStyle: "solid",
        borderRightWidth: "1px",
        display: "grid",
        paddingBlock: "28px",
        paddingInline: "24px",
    },
    calAvatar: {
        alignItems: "center",
        backgroundColor: palette.teal,
        borderRadius: "50%",
        color: "white",
        display: "flex",
        fontSize: "26px",
        fontWeight: 700,
        height: "64px",
        justifyContent: "center",
        marginBottom: "16px",
        width: "64px",
    },
    calName: {
        color: `color-mix(in srgb, ${calendly.ink} 61%, transparent)`,
        fontSize: "16px",
        fontWeight: 700,
    },
    calEvent: {
        color: calendly.navy,
        fontSize: "28px",
        fontWeight: 700,
        lineHeight: 1.25,
        marginBottom: "24px",
        marginTop: "4px",
    },
    calDetail: {
        alignItems: "flex-start",
        color: `color-mix(in srgb, ${calendly.ink} 61%, transparent)`,
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
        paddingBlock: "28px",
        paddingInlineStart: "24px",
        paddingInlineEnd: "20px",
    },
    calTitle: {
        color: calendly.navy,
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
        backgroundColor: `color-mix(in srgb, ${calendly.blue} 6.5%, transparent)`,
        borderRadius: "50%",
        color: calendly.deepBlue,
        display: "flex",
        height: "36px",
        justifyContent: "center",
        width: "36px",
    },
    calStepOff: {
        backgroundColor: "transparent",
        color: `color-mix(in srgb, ${calendly.ink} 40%, transparent)`,
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
        backgroundColor: `color-mix(in srgb, ${calendly.blue} 6.5%, transparent)`,
        color: calendly.deepBlue,
        fontWeight: 700,
    },
    calChosen: {
        backgroundColor: calendly.blue,
        color: "white",
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
        paddingBlockStart: "92px",
        paddingBlockEnd: "28px",
        paddingInlineStart: "0",
        paddingInlineEnd: "20px",
    },
    calDay: {
        fontSize: "16px",
        marginBottom: "6px",
    },
    calTime: {
        borderColor: `color-mix(in srgb, ${calendly.blue} 50%, transparent)`,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: calendly.blue,
        fontSize: "16px",
        fontWeight: 700,
        paddingBlock: "14px",
        textAlign: "center",
    },
    calRibbon: {
        backgroundColor: calendly.slate,
        color: "white",
        display: "grid",
        fontSize: "14px",
        fontWeight: 700,
        justifyItems: "center",
        lineHeight: 1.1,
        paddingBlockStart: "5px",
        paddingBlockEnd: "6px",
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
        backgroundColor: "white",
        color: notion.ink,
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
        color: notion.ink,
        display: "flex",
        gap: "16px",
    },
    notionButton: {
        backgroundColor: notion.charcoal,
        borderRadius: "6px",
        color: "white",
        fontSize: "14px",
        fontWeight: 500,
        paddingBlock: "6px",
        paddingInline: "10px",
    },
    notionCover: {
        backgroundImage: `linear-gradient(120deg, ${notion.paper} 0%, ${notion.tan} 50%, ${notion.clay} 100%)`,
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
        color: `color-mix(in srgb, ${notion.ink} 65%, transparent)`,
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
        backgroundColor: notion.sky,
        borderRadius: "10px",
        color: notion.navy,
        display: "inline-flex",
        gap: "6px",
        paddingBlock: "1px",
        paddingInlineStart: "8px",
        paddingInlineEnd: "9px",
    },
    notionDot: {
        backgroundColor: notion.blue,
        borderRadius: "50%",
        height: "8px",
        width: "8px",
    },
    notionPerson: {
        alignItems: "center",
        backgroundColor: palette.teal,
        borderRadius: "50%",
        color: "white",
        display: "flex",
        fontSize: "10px",
        fontWeight: 700,
        height: "20px",
        justifyContent: "center",
        marginRight: "6px",
        width: "20px",
    },
    notionRule: {
        borderColor: `color-mix(in srgb, ${notion.ink} 9%, transparent)`,
        borderStyle: "solid",
        borderBlockStartWidth: "1px",
        borderBlockEndWidth: "0",
        borderInlineWidth: "0",
        marginBlockStart: "16px",
        marginBlockEnd: "20px",
        marginInline: "0",
        width: "100%",
    },
    notionCallout: {
        alignItems: "flex-start",
        backgroundColor: notion.sand,
        borderRadius: "4px",
        display: "flex",
        fontSize: "16px",
        gap: "8px",
        lineHeight: 1.5,
        margin: 0,
        paddingBlock: "16px",
        paddingInlineStart: "12px",
        paddingInlineEnd: "16px",
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
        borderColor: notion.ink,
        borderRadius: "3px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        color: "white",
        display: "flex",
        flexShrink: 0,
        height: "16px",
        justifyContent: "center",
        width: "16px",
    },
    notionBoxDone: {
        backgroundColor: notion.link,
        borderColor: notion.link,
    },
    notionDone: {
        opacity: 0.375,
        textDecorationLine: "line-through",
    },
    linear: {
        backgroundColor: "white",
        color: linear.ink,
        display: "grid",
        fontFamily: faces.linear,
        fontSize: "13px",
        gridTemplateColumns: "220px minmax(0, 1fr)",
        minHeight: "100%",
    },
    linSide: {
        alignContent: "start",
        backgroundColor: linear.panel,
        borderRightColor: linear.rule,
        borderRightStyle: "solid",
        borderRightWidth: "1px",
        display: "grid",
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
        backgroundColor: "white",
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
        display: "flex",
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
        display: "flex",
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
