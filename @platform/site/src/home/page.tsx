import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { tokens } from "../style/tokens.stylex";
import { Fade } from "./fade";
import type { Item } from "./ledger";
import type { Stagger } from "./stagger";
import { Tile } from "./tile";

/** The twelve services a docs app runs on, numbered as the ledger rows the inspector lights: the services the app calls, then the infrastructure it runs on, rented one by one or built into one engine. */
export const services: readonly { stacked: Item; destacked: Item }[] = [
    {
        stacked: {
            label: "Identity",
            mark: { logo: "clerk.png" },
            name: "Clerk",
            chips: ["SDK", "webhook"],
            note: "users copied in by webhook",
        },
        destacked: {
            label: "Identity",
            mark: { icon: "user", tint: "#5b7f2e" },
            name: "Accounts",
            chips: ["passkeys"],
            note: "people and agents, one sign-in",
        },
    },
    {
        stacked: {
            label: "Permissions",
            mark: { logo: "workos.png" },
            name: "WorkOS",
            chips: ["SDK", "sync"],
            note: "a second permission model",
        },
        destacked: {
            label: "Permissions",
            mark: { icon: "auth", tint: "#a0485f" },
            name: "Access",
            chips: ["roles"],
            note: "one rule set for every app",
        },
    },
    {
        stacked: {
            label: "Secrets",
            mark: { logo: "doppler.png" },
            name: "Doppler",
            chips: ["CLI"],
            note: "keys pasted into every service",
        },
        destacked: {
            label: "Secrets",
            mark: { icon: "vault", tint: "#12313c" },
            name: "Vault",
            chips: ["encrypted"],
            note: "keys that never leave the server",
        },
    },
    {
        stacked: {
            label: "Database",
            mark: { logo: "supabase.svg" },
            name: "Supabase",
            chips: ["realtime"],
            note: "a second backend to keep in step",
        },
        destacked: {
            label: "Database",
            mark: { icon: "storage", tint: "#2f7d8c" },
            name: "Database",
            chips: ["SQL", "live"],
            note: "live queries, offline too",
        },
    },
    {
        stacked: {
            label: "Files",
            mark: { logo: "dropbox.svg" },
            name: "Dropbox",
            chips: ["embed"],
            note: "files kept outside the page",
        },
        destacked: {
            label: "Files",
            mark: { icon: "bucket", tint: "#2f7d8c" },
            name: "Files",
            chips: ["blobs"],
            note: "files attached where they're used",
        },
    },
    {
        stacked: {
            label: "Search",
            mark: { logo: "elastic.svg" },
            name: "Elastic",
            chips: ["SDK", "sync"],
            note: "a second index to keep in sync",
        },
        destacked: {
            label: "Search",
            mark: { icon: "search", tint: "#3d6fb0" },
            name: "Search",
            chips: ["full text"],
            note: "one index over every app",
        },
    },
    {
        stacked: {
            label: "Messaging",
            mark: { logo: "resend.svg" },
            name: "Resend",
            chips: ["SDK"],
            note: "an extra SDK just for email",
        },
        destacked: {
            label: "Messaging",
            mark: { icon: "notify", tint: "#b8862b" },
            name: "Notifications",
            chips: ["push"],
            note: "one inbox for every app",
        },
    },
    {
        stacked: {
            label: "Jobs",
            mark: { logo: "temporal.png" },
            name: "Temporal",
            chips: ["workers"],
            note: "a worker fleet beside the app",
        },
        destacked: {
            label: "Jobs",
            mark: { icon: "sync", tint: "#4f8a5b" },
            name: "Workflows",
            chips: ["durable"],
            note: "runs that survive restarts",
        },
    },
    {
        stacked: {
            label: "Analytics",
            mark: { logo: "posthog.svg" },
            name: "PostHog",
            chips: ["snippet"],
            note: "events sent to their cloud",
        },
        destacked: {
            label: "Analytics",
            mark: { icon: "telemetry", tint: "#6d7f86" },
            name: "Telemetry",
            chips: ["events"],
            note: "events kept with your data",
        },
    },
    {
        stacked: {
            label: "Errors",
            mark: { logo: "sentry.png" },
            name: "Sentry",
            chips: ["SDK"],
            note: "errors seen one SDK at a time",
        },
        destacked: {
            label: "Errors",
            mark: { icon: "telemetry", tint: "#6d7f86" },
            name: "Telemetry",
            chips: ["traces"],
            note: "every call traced end to end",
        },
    },
    {
        stacked: {
            label: "Hosting",
            mark: { logo: "vercel.png" },
            name: "Vercel",
            chips: ["config"],
            note: "runs only in their cloud",
        },
        destacked: {
            label: "Hosting",
            mark: { icon: "hosts", tint: "#4f8a5b" },
            name: "Hosting",
            chips: ["Node"],
            note: "your laptop, server or cloud",
        },
    },
    {
        stacked: {
            label: "Code",
            mark: { logo: "github.png" },
            name: "GitHub",
            chips: ["CI"],
            note: "a pipeline of its own",
        },
        destacked: {
            label: "Code",
            mark: { icon: "source", tint: "#c64a17" },
            name: "Forge",
            chips: ["git"],
            note: "every app's source, forkable",
        },
    },
];

/** The checklist before launch: each item and whether it is done. */
const checklist: readonly (readonly [item: string, isDone: boolean])[] = [
    ["Freeze the pricing page", true],
    ["Rehearse the demo", false],
    ["Schedule the announcement", false],
];

/** The other apps beside Pages in its rail: rented from other vendors, each opening elsewhere, or in your space, drawn alike. */
const railApps: Record<
    "stacked" | "destacked",
    readonly (readonly [name: string, label: string])[]
> = {
    stacked: [
        ["linear", "Linear"],
        ["slack", "Slack"],
        ["github", "GitHub"],
        ["figma", "Figma"],
    ],
    destacked: [
        ["tasks", "Tasks"],
        ["calendar", "My week"],
        ["mail", "Mail"],
        ["bucket", "Files"],
    ],
};

/** The tasks blocked by another once owned, in the column you added: the task and its blocker. */
const blockers: Readonly<Record<string, string>> = { "LCH-13": "LCH-15", "LCH-16": "LCH-14" };

/** How far a task has come. */
type Status = "todo" | "started" | "done";

/** A task's change of code: its pull request while rented, its branch once owned, and whether its checks pass. */
type Change = readonly [pull: string, branch: string, isPassing: boolean];

/** The launch tasks in the page's table: id, title, status, assignee and their tint, due date, and change of code. */
const tasks: readonly (readonly [
    id: string,
    title: string,
    status: Status,
    assignee: string,
    tint: string,
    due: string,
    change?: Change,
])[] = [
    ["LCH-12", "Pricing page", "done", "Me", "#2f7d8c", "Mon 20", ["#408", "pricing-page", true]],
    [
        "LCH-13",
        "Turn on the new prices",
        "todo",
        "Me",
        "#6b5ca5",
        "Fri 24",
        ["#412", "new-prices", false],
    ],
    ["LCH-14", "Record the demo", "started", "Me", "#2f7d8c", "Thu 23"],
    ["LCH-15", "Draft the launch post", "todo", "Agent", "#b8862b", "Thu 23"],
    ["LCH-16", "Email the waitlist", "todo", "Agent", "#6b5ca5", "Fri 24"],
];

/** Say who can see the task table: everyone by one access rule, or not your agent without a seat. */
function Access(properties: { isOpen: boolean }) {
    return (
        <span
            data-component={properties.isOpen ? "AccessRule" : "SeatWall"}
            data-service="2"
            {...stylex.attrs(styles.access)}
        >
            <span
                style={{
                    "mask-image": `url(/diagram/${properties.isOpen ? "auth" : "lock"}.svg)`,
                }}
                {...stylex.attrs(styles.glyph)}
            />
            {properties.isOpen ? "You and your agent" : "No seat for your agent"}
        </span>
    );
}

/** Name the state of a change's checks: as last synced while rented, live once owned. */
function checksOf(change: Change, isOpen: boolean) {
    if (isOpen) {
        return change[2] ? "passed" : "running";
    }

    return change[2] ? "merged" : "CI failing";
}

/**
 * Draw the Pages app with the launch plan open, every part named for the inspector, and the twelve services marked where they show.
 *
 * While rented, the page shows where its vendors meet: vendor apps in its rail, an edit conflict, a key to paste, a failed automation, an embed behind seats, and a preview on another site.
 * Once owned, the same page sits among your other apps and holds its tasks, prices, agent, code and previews natively, with a column you added, each switching on the step of its service.
 */
export function PagesApp(properties: { isOpenAt: Stagger }) {
    return (
        <div {...stylex.attrs(styles.app)}>
            <nav data-component="AppRail" {...stylex.attrs(styles.rail)}>
                <span
                    data-component="AccountMenu"
                    data-service="1"
                    title="Your space"
                    {...stylex.attrs(styles.avatar, styles.railAvatar)}
                    style={{ "background-color": "#2f7d8c" }}
                >
                    F
                </span>
                <span
                    data-component="SearchButton"
                    data-service="6"
                    title="Search"
                    {...stylex.attrs(styles.railButton)}
                >
                    <span
                        style={{ "mask-image": "url(/diagram/search.svg)" }}
                        {...stylex.attrs(styles.glyph)}
                    />
                </span>
                <span
                    data-component="InboxButton"
                    data-service="7"
                    title="Inbox"
                    {...stylex.attrs(styles.railButton)}
                >
                    <span
                        style={{ "mask-image": "url(/diagram/notify.svg)" }}
                        {...stylex.attrs(styles.glyph)}
                    />
                    <span {...stylex.attrs(styles.badge, styles.railBadge)}>3</span>
                </span>
                <span aria-hidden="true" {...stylex.attrs(styles.railRule)} />
                <span
                    data-component="PagesButton"
                    title="Pages"
                    {...stylex.attrs(styles.railButton, styles.selected)}
                >
                    {properties.isOpenAt(1) ? (
                        <span
                            style={{ "mask-image": "url(/diagram/pages.svg)" }}
                            {...stylex.attrs(styles.glyph, styles.glyphOn)}
                        />
                    ) : (
                        <Tile name="pages" />
                    )}
                </span>
                <Fade
                    isOpen={properties.isOpenAt(1)}
                    component={properties.isOpenAt(1) ? "SpaceApps" : "VendorApps"}
                    style={styles.railApps}
                >
                    {railApps[properties.isOpenAt(1) ? "destacked" : "stacked"].map(
                        ([name, label]) => (
                            <span
                                data-component="AppButton"
                                data-service="1"
                                title={label}
                                {...stylex.attrs(styles.railButton)}
                            >
                                {properties.isOpenAt(1) ? (
                                    <span
                                        style={{ "mask-image": `url(/diagram/${name}.svg)` }}
                                        {...stylex.attrs(styles.glyph)}
                                    />
                                ) : (
                                    <>
                                        <Tile name={name} />
                                        <span {...stylex.attrs(styles.away)}>↗</span>
                                    </>
                                )}
                            </span>
                        ),
                    )}
                </Fade>
            </nav>
            <div {...stylex.attrs(styles.main)}>
                <article data-component="Page" {...stylex.attrs(styles.page)}>
                    <div data-component="PageHeader" {...stylex.attrs(styles.toolbar)}>
                        <b data-component="PageTitle" {...stylex.attrs(styles.title)}>
                            Launch plan
                        </b>
                        <div {...stylex.attrs(styles.tools)}>
                            <Fade
                                isOpen={properties.isOpenAt(11)}
                                style={styles.roomy}
                                component={
                                    properties.isOpenAt(11) ? "PreviewButton" : "VercelPreviewLink"
                                }
                            >
                                <span data-service="11" {...stylex.attrs(styles.preview)}>
                                    {properties.isOpenAt(11)
                                        ? "Preview branch"
                                        : "Preview on Vercel ↗"}
                                </span>
                            </Fade>
                            <Fade
                                isOpen={properties.isOpenAt(12)}
                                component={
                                    properties.isOpenAt(12) ? "EditAppButton" : "FeedbackLink"
                                }
                            >
                                <span
                                    data-service="12"
                                    {...stylex.attrs(
                                        styles.preview,
                                        properties.isOpenAt(12) && styles.editApp,
                                    )}
                                >
                                    {properties.isOpenAt(12) ? "Edit app" : "Send feedback"}
                                </span>
                            </Fade>
                            <span
                                data-component="ShareButton"
                                data-service="2"
                                {...stylex.attrs(styles.share)}
                            >
                                Share
                            </span>
                        </div>
                    </div>
                    <div data-component="PropertyList" {...stylex.attrs(styles.properties)}>
                        <span data-component="StatusProperty" {...stylex.attrs(styles.property)}>
                            <span {...stylex.attrs(styles.state, styles.started, styles.flush)} />
                            In progress
                        </span>
                        <span data-component="DateProperty" {...stylex.attrs(styles.property)}>
                            Fri 24 Oct
                        </span>
                        <Fade
                            isOpen={properties.isOpenAt(3)}
                            component={properties.isOpenAt(3) ? "PriceProperty" : "StripeKeyPrompt"}
                            style={styles.slot}
                        >
                            <span
                                data-service="3"
                                {...stylex.attrs(
                                    styles.property,
                                    !properties.isOpenAt(3) && styles.propertyMissing,
                                )}
                            >
                                <span
                                    style={{ "mask-image": "url(/diagram/vault.svg)" }}
                                    {...stylex.attrs(styles.glyph)}
                                />
                                {properties.isOpenAt(3)
                                    ? "Pro €12/mo · live"
                                    : "Paste a Stripe API key to show prices"}
                            </span>
                        </Fade>
                        <span
                            data-component="ScheduleProperty"
                            data-service="8"
                            {...stylex.attrs(styles.property)}
                        >
                            Reminder Thu
                        </span>
                    </div>
                    <p data-component="Paragraph" {...stylex.attrs(styles.text)}>
                        Ship to the waitlist on Thursday.{" "}
                        <span data-component="Presence">
                            Pricing stays in step with billing
                            <span data-component="Cursor" {...stylex.attrs(styles.cursor)}>
                                <span {...stylex.attrs(styles.flag)}>Agent</span>
                            </span>
                        </span>
                    </p>
                    <Fade
                        isOpen={properties.isOpenAt(8)}
                        component={properties.isOpenAt(8) ? "AgentNote" : "AutomationError"}
                    >
                        <p
                            data-service="8"
                            {...stylex.attrs(
                                styles.agentNote,
                                !properties.isOpenAt(8) && styles.error,
                            )}
                        >
                            {properties.isOpenAt(8) ? (
                                <>
                                    <Tile name="agent" />
                                    <span>
                                        <b {...stylex.attrs(styles.strong)}>Agent</b>{" "}
                                        <span {...stylex.attrs(styles.quiet)}>
                                            drafted LCH-15 for you to review
                                        </span>
                                    </span>
                                </>
                            ) : (
                                <>
                                    <img
                                        alt=""
                                        src="/logos/temporal.png"
                                        {...stylex.attrs(styles.logo)}
                                    />
                                    <span>
                                        <b {...stylex.attrs(styles.strong)}>Run failed</b>{" "}
                                        <span>Draft the launch post · retrying in 5 min</span>
                                    </span>
                                </>
                            )}
                        </p>
                    </Fade>
                    <Fade
                        isOpen={properties.isOpenAt(4)}
                        component={properties.isOpenAt(4) ? "TaskTable" : "LinearEmbed"}
                        style={properties.isOpenAt(4) ? styles.tasks : styles.embed}
                        isLarge
                    >
                        <p data-service="4" {...stylex.attrs(styles.tableBar)}>
                            {properties.isOpenAt(4) ? (
                                <>
                                    <Tile name="tasks" />
                                    <b {...stylex.attrs(styles.strong)}>Launch tasks</b>
                                    <span {...stylex.attrs(styles.quiet, styles.wide)}>
                                        from Tasks, also in My week
                                    </span>
                                    <span {...stylex.attrs(styles.tools)}>
                                        <Access isOpen={true} />
                                        <span {...stylex.attrs(styles.newTask)}>+ New task</span>
                                    </span>
                                </>
                            ) : (
                                <>
                                    <Tile name="linear" />
                                    <b {...stylex.attrs(styles.strong)}>Linear embed</b>
                                    <span {...stylex.attrs(styles.quiet, styles.wide)}>
                                        Read-only · synced 3 h ago
                                    </span>
                                    <span {...stylex.attrs(styles.tools)}>
                                        <Access isOpen={false} />
                                        <span {...stylex.attrs(styles.failing)}>Reconnect</span>
                                    </span>
                                </>
                            )}
                        </p>
                        <table
                            data-service="4"
                            {...stylex.attrs(styles.table, !properties.isOpenAt(4) && styles.stale)}
                        >
                            <thead>
                                <tr>
                                    <th {...stylex.attrs(styles.head, styles.id)}>ID</th>
                                    <th {...stylex.attrs(styles.head)}>Task</th>
                                    <th {...stylex.attrs(styles.head, styles.wide)}>Owner</th>
                                    <th {...stylex.attrs(styles.head, styles.wide)}>Due</th>
                                    <th {...stylex.attrs(styles.head, styles.wide)}>
                                        {properties.isOpenAt(12) ? "Checks" : "Pull request"}
                                    </th>
                                    {properties.isOpenAt(4) && properties.isOpenAt(12) ? (
                                        <th
                                            data-component="AddedColumn"
                                            data-service="12"
                                            {...stylex.attrs(
                                                styles.head,
                                                styles.added,
                                                styles.wide,
                                            )}
                                        >
                                            Blocked by
                                            <span {...stylex.attrs(styles.yours)}>yours</span>
                                        </th>
                                    ) : undefined}
                                </tr>
                            </thead>
                            <tbody>
                                {tasks.map(([id, title, status, assignee, tint, due, change]) => (
                                    <tr data-component="ObjectRow">
                                        <td {...stylex.attrs(styles.cell, styles.id)}>{id}</td>
                                        <td {...stylex.attrs(styles.cell, styles.taskTitle)}>
                                            <span {...stylex.attrs(styles.state, styles[status])} />
                                            {title}
                                        </td>
                                        <td {...stylex.attrs(styles.cell, styles.wide)}>
                                            <span {...stylex.attrs(styles.row)}>
                                                <span
                                                    data-component="Avatar"
                                                    {...stylex.attrs(styles.avatar)}
                                                    style={{ "background-color": tint }}
                                                >
                                                    {assignee === "Me" ? "F" : assignee.charAt(0)}
                                                </span>
                                            </span>
                                        </td>
                                        <td
                                            {...stylex.attrs(
                                                styles.cell,
                                                styles.quiet,
                                                styles.wide,
                                            )}
                                        >
                                            {due}
                                        </td>
                                        <td
                                            data-component={
                                                properties.isOpenAt(12)
                                                    ? "BranchCell"
                                                    : "PullRequestCell"
                                            }
                                            data-service="12"
                                            {...stylex.attrs(
                                                styles.cell,
                                                styles.change,
                                                styles.wide,
                                            )}
                                        >
                                            {change === undefined ? undefined : (
                                                <span {...stylex.attrs(styles.row)}>
                                                    <span
                                                        style={{
                                                            "mask-image": `url(/diagram/${properties.isOpenAt(12) ? "source" : "github"}.svg)`,
                                                        }}
                                                        {...stylex.attrs(styles.glyph)}
                                                    />
                                                    {properties.isOpenAt(12)
                                                        ? undefined
                                                        : `${change[0]} ↗`}
                                                    <span
                                                        {...stylex.attrs(
                                                            change[2]
                                                                ? styles.passing
                                                                : properties.isOpenAt(12)
                                                                  ? styles.running
                                                                  : styles.failing,
                                                            !properties.isOpenAt(12) &&
                                                                change[2] &&
                                                                styles.quiet,
                                                        )}
                                                    >
                                                        {checksOf(change, properties.isOpenAt(12))}
                                                    </span>
                                                </span>
                                            )}
                                        </td>
                                        {properties.isOpenAt(4) && properties.isOpenAt(12) ? (
                                            <td
                                                data-component="BlockerCell"
                                                data-service="12"
                                                {...stylex.attrs(
                                                    styles.cell,
                                                    styles.id,
                                                    styles.addedCell,
                                                    styles.wide,
                                                )}
                                            >
                                                {blockers[id] ?? "—"}
                                            </td>
                                        ) : undefined}
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </Fade>
                    <b data-component="Heading" {...stylex.attrs(styles.heading)}>
                        Notes from Monday
                    </b>
                    <p data-component="Paragraph" {...stylex.attrs(styles.text)}>
                        The waitlist has 1,840 people. Your agent sends the first batch of invites
                        on Thursday morning, and the rest follow once the demo is live.
                    </p>
                    <p data-component="Attachment" data-service="5" {...stylex.attrs(styles.file)}>
                        <span
                            style={{ "mask-image": "url(/diagram/file.svg)" }}
                            {...stylex.attrs(styles.glyph)}
                        />
                        pricing-v4.pdf
                        <span {...stylex.attrs(styles.quiet)}>
                            {properties.isOpenAt(5)
                                ? "1.2 MB · added by you"
                                : "Dropbox ↗ · request access"}
                        </span>
                    </p>
                    <p data-component="Comment" {...stylex.attrs(styles.comment)}>
                        <span
                            data-component="Avatar"
                            {...stylex.attrs(styles.avatar)}
                            style={{ "background-color": "#6b5ca5" }}
                        >
                            A
                        </span>
                        <span>
                            <b {...stylex.attrs(styles.strong)}>Agent</b>{" "}
                            <span {...stylex.attrs(styles.quiet)}>
                                The launch post draft is ready for you
                            </span>
                        </span>
                    </p>
                    <p data-component="Paragraph" {...stylex.attrs(styles.text)}>
                        Pricing launches with three plans. The team plan replaces the old seat
                        pricing, and existing customers keep their rate for a year.
                    </p>
                    <b data-component="Heading" {...stylex.attrs(styles.heading)}>
                        Before launch
                    </b>
                    <ul data-component="Checklist" {...stylex.attrs(styles.checklist)}>
                        {checklist.map(([item, isDone]) => (
                            <li data-component="CheckItem" {...stylex.attrs(styles.check)}>
                                <span {...stylex.attrs(styles.box, isDone && styles.boxDone)} />
                                <span {...stylex.attrs(isDone && styles.struck)}>{item}</span>
                            </li>
                        ))}
                    </ul>
                </article>
            </div>
            <div data-component="StatusBar" {...stylex.attrs(styles.statusBar)}>
                <span
                    data-component="SyncStatus"
                    data-service="11"
                    {...stylex.attrs(styles.status)}
                >
                    <span
                        {...stylex.attrs(styles.dot, !properties.isOpenAt(11) && styles.dotCloud)}
                    />
                    {properties.isOpenAt(11) ? "Yours · on your laptop" : "Saved to their cloud"}
                </span>
                <span
                    data-component={properties.isOpenAt(4) ? "PresenceList" : "ConflictNotice"}
                    data-service="4"
                    {...stylex.attrs(styles.status, styles.wide)}
                >
                    <span
                        {...stylex.attrs(styles.avatar, styles.small)}
                        style={{ "background-color": "#2f7d8c" }}
                    >
                        F
                    </span>
                    <span
                        {...stylex.attrs(styles.avatar, styles.small)}
                        style={{ "background-color": "#6b5ca5" }}
                    >
                        A
                    </span>
                    {properties.isOpenAt(4) ? (
                        <span {...stylex.attrs(styles.presenceText)}>
                            You and your agent editing
                        </span>
                    ) : (
                        <span {...stylex.attrs(styles.presenceText)}>
                            Agent's edits
                            <span {...stylex.attrs(styles.reload)}>Reload</span>
                        </span>
                    )}
                </span>
                <span {...stylex.attrs(styles.statusEnd)}>
                    <span
                        data-component={properties.isOpenAt(9) ? "ViewCount" : "AnalyticsSnippet"}
                        data-service="9"
                        {...stylex.attrs(styles.status, styles.wide)}
                    >
                        {properties.isOpenAt(9) ? "182 views" : "PostHog ↗"}
                    </span>
                    <span
                        data-component="ErrorCount"
                        data-service="10"
                        {...stylex.attrs(styles.status, !properties.isOpenAt(10) && styles.failing)}
                    >
                        {properties.isOpenAt(10) ? "0 errors" : "3 integrations failing"}
                    </span>
                    <span
                        data-component="BranchMenu"
                        data-service="12"
                        {...stylex.attrs(styles.status, styles.wide)}
                    >
                        <span
                            style={{ "mask-image": "url(/diagram/source.svg)" }}
                            {...stylex.attrs(styles.glyph)}
                        />
                        main
                    </span>
                </span>
            </div>
        </div>
    );
}

/** The Pages app styles. */
const styles = stylex.create({
    app: {
        display: "grid",
        fontSize: "0.8125rem",
        gridTemplateColumns: "3rem minmax(0, 1fr)",
        gridTemplateRows: "minmax(0, 1fr) auto",
        height: "100%",
        minHeight: 0,
        "@media (max-width: 767px)": { gridTemplateColumns: "minmax(0, 1fr)" },
    },
    rail: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRightColor: tokens.rule,
        borderRightStyle: "solid",
        borderRightWidth: tokens.hairline,
        display: "flex",
        flexDirection: "column",
        gap: "0.5rem",
        paddingBlock: "0.75rem",
        "@media (max-width: 767px)": { display: "none" },
    },
    railButton: {
        alignItems: "center",
        borderRadius: "6px",
        display: "flex",
        flexShrink: 0,
        height: "2rem",
        justifyContent: "center",
        position: "relative",
        width: "2rem",
    },
    railAvatar: {
        height: "1.625rem",
        marginBottom: "0.25rem",
        width: "1.625rem",
    },
    railBadge: {
        fontSize: "0.55rem",
        paddingInline: "0.25rem",
        position: "absolute",
        right: "-0.125rem",
        top: "-0.125rem",
    },
    agentNote: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: "6px",
        display: "flex",
        gap: "0.625rem",
        margin: 0,
        paddingBlock: "0.375rem",
        paddingInline: "0.625rem",
    },
    tasks: {
        borderColor: tokens.rule,
        borderRadius: "8px",
        overflow: "hidden",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "grid",
    },
    embed: {
        borderColor: tokens.rule,
        borderRadius: "8px",
        overflow: "hidden",
        borderStyle: "dashed",
        borderWidth: tokens.hairline,
        display: "grid",
    },
    tableBar: {
        alignItems: "center",
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "flex",
        gap: "0.5rem",
        margin: 0,
        paddingBlock: "0.5rem",
        paddingInline: "0.75rem",
        whiteSpace: "nowrap",
    },
    editApp: {
        color: color.foreground,
        fontWeight: 600,
    },
    glyphOn: {
        backgroundColor: tokens.signal,
    },
    away: {
        backgroundColor: color.card,
        borderRadius: "50%",
        bottom: "-0.0625rem",
        color: color.mutedForeground,
        fontSize: "0.55rem",
        fontWeight: 700,
        lineHeight: "0.75rem",
        position: "absolute",
        right: "-0.0625rem",
        textAlign: "center",
        width: "0.75rem",
    },
    railRule: {
        backgroundColor: tokens.rule,
        flexShrink: 0,
        height: tokens.hairline,
        marginBlock: "0.125rem",
        width: "1.5rem",
    },
    railApps: {
        display: "grid",
        flexShrink: 0,
        gap: "0.5rem",
    },
    head: {
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        color: color.mutedForeground,
        fontSize: "0.68rem",
        fontWeight: 500,
        paddingBlock: "0.375rem",
        paddingRight: "0.75rem",
        textAlign: "left",
        whiteSpace: "nowrap",
    },
    added: {
        color: color.foreground,
        fontWeight: 600,
        paddingLeft: "0.5rem",
    },
    addedCell: {
        paddingLeft: "0.5rem",
    },
    yours: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.58rem",
        fontWeight: 400,
        letterSpacing: "0.06em",
        marginLeft: "0.375rem",
        textTransform: "uppercase",
    },
    roomy: {
        display: { default: "block", "@media (max-width: 767px)": "none" },
    },
    slot: {
        display: "flex",
    },
    presenceText: {
        marginLeft: "0.375rem",
    },
    preview: {
        color: color.mutedForeground,
        display: "block",
        lineHeight: "1.25rem",
        whiteSpace: "nowrap",
    },
    reload: {
        color: color.foreground,
        fontWeight: 600,
        marginLeft: "0.5rem",
    },
    access: {
        alignItems: "center",
        color: color.mutedForeground,
        display: { default: "flex", "@media (max-width: 767px)": "none" },
        gap: "0.375rem",
    },
    change: {
        fontSize: "0.75rem",
    },
    passing: {
        color: "#3c8f58",
        fontWeight: 600,
    },
    running: {
        color: "#b8862b",
        fontWeight: 600,
    },
    newTask: {
        borderColor: tokens.rule,
        borderRadius: "5px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        color: color.foreground,
        fontWeight: 600,
        paddingBlock: "0.125rem",
        paddingInline: "0.5rem",
    },
    failing: {
        color: "#c0392b",
        fontWeight: 600,
    },
    stale: {
        opacity: 0.55,
    },
    error: {
        backgroundColor: "rgb(192 57 43 / 9%)",
        color: "#b03a2e",
    },
    logo: {
        borderRadius: "5px",
        flexShrink: 0,
        height: "1.25rem",
        width: "1.25rem",
    },
    propertyMissing: {
        backgroundColor: "transparent",
        borderColor: color.mutedForeground,
        borderStyle: "dashed",
        borderWidth: tokens.hairline,
    },
    dotCloud: {
        backgroundColor: color.mutedForeground,
    },
    row: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
    },
    avatar: {
        alignItems: "center",
        borderRadius: "50%",
        color: "#ffffff",
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "0.6rem",
        fontWeight: 700,
        height: "1.125rem",
        justifyContent: "center",
        width: "1.125rem",
    },
    glyph: {
        backgroundColor: color.mutedForeground,
        flexShrink: 0,
        height: "0.875rem",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "0.875rem",
    },
    badge: {
        backgroundColor: tokens.signal,
        borderRadius: "999px",
        color: tokens.signalInk,
        fontSize: "0.62rem",
        fontWeight: 700,
        paddingInline: "0.375rem",
    },
    heading: {
        fontSize: "1rem",
        fontWeight: 700,
        marginTop: "0.25rem",
    },
    file: {
        alignItems: "center",
        alignSelf: "start",
        borderColor: tokens.rule,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "inline-flex",
        gap: "0.5rem",
        justifySelf: "start",
        margin: 0,
        paddingBlock: "0.375rem",
        paddingInline: "0.625rem",
    },
    checklist: {
        display: "grid",
        gap: "0.375rem",
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    check: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
    },
    box: {
        borderColor: color.mutedForeground,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        flexShrink: 0,
        height: "0.875rem",
        width: "0.875rem",
    },
    boxDone: {
        backgroundColor: "#5e6ad2",
        borderColor: "#5e6ad2",
    },
    struck: {
        color: color.mutedForeground,
        textDecorationLine: "line-through",
    },
    comment: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
        margin: 0,
    },
    selected: {
        backgroundColor: "rgb(255 121 46 / 14%)",
    },
    main: {
        display: "grid",
        minHeight: 0,
        minWidth: 0,
    },
    toolbar: {
        alignItems: "center",
        display: "flex",
        flexWrap: "wrap",
        gap: "0.5rem 1rem",
        margin: 0,
    },
    tools: {
        alignItems: "center",
        display: "flex",
        gap: "1.25rem",
        marginLeft: "auto",
    },
    share: {
        backgroundColor: color.foreground,
        borderRadius: "5px",
        color: color.background,
        fontWeight: 600,
        paddingBlock: "0.125rem",
        paddingInline: "0.625rem",
    },
    page: {
        alignContent: "start",
        display: "grid",
        gap: "1.125rem",
        gridAutoRows: "max-content",
        gridTemplateColumns: "minmax(0, 1fr)",
        maskImage: "linear-gradient(to bottom, #000 calc(100% - 3rem), transparent)",
        minHeight: 0,
        minWidth: 0,
        overflow: "hidden",
        paddingBlock: "1.5rem",
        paddingInline: { default: "2rem", "@media (max-width: 767px)": "1rem" },
    },
    title: {
        fontSize: { default: "1.75rem", "@media (max-width: 767px)": "1.375rem" },
        fontWeight: 700,
        letterSpacing: "-0.02em",
        lineHeight: 1.15,
    },
    properties: {
        alignItems: "center",
        display: "flex",
        flexWrap: "wrap",
        gap: "0.5rem",
        margin: 0,
    },
    property: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: "5px",
        color: color.mutedForeground,
        display: "inline-flex",
        fontSize: "0.75rem",
        gap: "0.375rem",
        paddingBlock: "0.125rem",
        paddingInline: "0.5rem",
        whiteSpace: "nowrap",
    },
    text: {
        lineHeight: 1.6,
        margin: 0,
    },
    cursor: {
        backgroundColor: "#6b5ca5",
        display: "inline-block",
        height: "1.15em",
        marginLeft: "1px",
        position: "relative",
        verticalAlign: "-0.2em",
        width: "2px",
    },
    flag: {
        backgroundColor: "#6b5ca5",
        borderRadius: "3px 3px 3px 0",
        bottom: "100%",
        color: "#ffffff",
        fontSize: "0.6rem",
        fontWeight: 600,
        left: 0,
        lineHeight: "0.875rem",
        paddingInline: "0.25rem",
        position: "absolute",
        whiteSpace: "nowrap",
    },
    table: {
        borderCollapse: "collapse",
        marginBottom: "-1px",
        width: "100%",
    },
    cell: {
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        paddingBlock: "0.625rem",
        paddingRight: "0.75rem",
        whiteSpace: "nowrap",
        ":last-child": { paddingRight: "0.75rem" },
    },
    id: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.7rem",
        paddingLeft: "0.75rem",
        width: "4.25rem",
    },
    taskTitle: {
        fontWeight: 500,
    },
    state: {
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1.5px",
        display: "inline-block",
        flexShrink: 0,
        height: "0.6875rem",
        marginRight: "0.5rem",
        verticalAlign: "-0.0625rem",
        width: "0.6875rem",
    },
    todo: {
        borderColor: color.mutedForeground,
    },
    started: {
        backgroundImage: "linear-gradient(90deg, #d9a21b 50%, transparent 50%)",
        borderColor: "#d9a21b",
    },
    flush: {
        marginRight: 0,
    },
    done: {
        backgroundColor: "#5e6ad2",
        borderColor: "#5e6ad2",
    },
    strong: {
        fontWeight: 600,
    },
    quiet: {
        color: color.mutedForeground,
    },
    wide: {
        "@media (max-width: 767px)": { display: "none" },
    },
    statusBar: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderTopColor: tokens.rule,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        boxSizing: "border-box",
        color: color.mutedForeground,
        display: "flex",
        fontSize: "0.75rem",
        gap: "1.25rem",
        gridColumn: "1 / -1",
        height: "2.25rem",
        margin: 0,
        paddingInline: "0.875rem",
        whiteSpace: "nowrap",
    },
    status: {
        alignItems: "center",
        display: "flex",
        gap: "0.375rem",
    },
    statusEnd: {
        alignItems: "center",
        display: "flex",
        gap: "1.25rem",
        marginLeft: "auto",
    },
    small: {
        fontSize: "0.5rem",
        height: "0.9375rem",
        marginRight: "-0.25rem",
        width: "0.9375rem",
    },
    dot: {
        backgroundColor: "#4caf6e",
        borderRadius: "50%",
        height: "0.5rem",
        width: "0.5rem",
    },
});
