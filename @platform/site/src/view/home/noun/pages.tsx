import { color, font, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";

import { Fade } from "../figure/fade";
import type { Item } from "../figure/ledger";
import type { Stagger } from "../figure/stagger";
import { Tile } from "../figure/tile";
import { palette } from "../../palette.stylex";
import { appStyles, appText } from "../figure/app";
import { media } from "@destack/style/media.stylex";

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
            note: "users kept with your own data",
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
            note: "one permission model for everything",
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
            note: "keys kept in one vault",
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
            note: "one backend, always in step",
        },
    },
    {
        stacked: {
            label: "Files",
            mark: { logo: "dropbox.svg" },
            name: "Dropbox",
            chips: ["embed"],
            note: "files kept in their storage",
        },
        destacked: {
            label: "Files",
            mark: { icon: "bucket", tint: "#2f7d8c" },
            name: "Files",
            chips: ["blobs"],
            note: "files kept in your own storage",
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
            note: "one index, always in sync",
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
            note: "one notification system for all apps",
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
            note: "workflows running inside the app",
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
            note: "events kept in your own space",
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
            note: "errors traced across all apps",
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
            note: "runs on your laptop, server or cloud",
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
            note: "one pipeline for all apps",
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
            {...style.attrs(styles.access)}
        >
            <span
                style={{
                    "mask-image": `url(/diagram/${properties.isOpen ? "auth" : "lock"}.svg)`,
                }}
                {...style.attrs(appStyles.glyph)}
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
        <div {...style.attrs(appStyles.app)}>
            <nav data-component="AppRail" {...style.attrs(appStyles.rail)}>
                <span
                    data-component="AccountMenu"
                    data-service="1"
                    title="Your space"
                    {...style.attrs(appStyles.avatar, appStyles.railAvatar)}
                    style={{ "background-color": "#2f7d8c" }}
                >
                    F
                </span>
                <span
                    data-component="SearchButton"
                    data-service="6"
                    title="Search"
                    {...style.attrs(appStyles.railButton)}
                >
                    <span
                        style={{ "mask-image": "url(/diagram/search.svg)" }}
                        {...style.attrs(appStyles.glyph)}
                    />
                </span>
                <span
                    data-component="InboxButton"
                    data-service="7"
                    title="Inbox"
                    {...style.attrs(appStyles.railButton)}
                >
                    <span
                        style={{ "mask-image": "url(/diagram/notify.svg)" }}
                        {...style.attrs(appStyles.glyph)}
                    />
                    <span {...style.attrs(appStyles.badge, appStyles.railBadge)}>3</span>
                </span>
                <span aria-hidden="true" {...style.attrs(appStyles.railRule)} />
                <span
                    data-component="PagesButton"
                    title="Pages"
                    {...style.attrs(appStyles.railButton, appStyles.selected)}
                >
                    {properties.isOpenAt(1) ? (
                        <span
                            style={{ "mask-image": "url(/diagram/pages.svg)" }}
                            {...style.attrs(appStyles.glyph, appStyles.glyphOn)}
                        />
                    ) : (
                        <Tile name="pages" />
                    )}
                </span>
                <Fade
                    isOpen={properties.isOpenAt(1)}
                    component={properties.isOpenAt(1) ? "SpaceApps" : "VendorApps"}
                    xstyle={styles.railApps}
                >
                    {railApps[properties.isOpenAt(1) ? "destacked" : "stacked"].map(
                        ([name, label]) => (
                            <span
                                data-component="AppButton"
                                data-service="1"
                                title={label}
                                {...style.attrs(appStyles.railButton)}
                            >
                                {properties.isOpenAt(1) ? (
                                    <span
                                        style={{ "mask-image": `url(/diagram/${name}.svg)` }}
                                        {...style.attrs(appStyles.glyph)}
                                    />
                                ) : (
                                    <>
                                        <Tile name={name} />
                                        <span {...style.attrs(styles.away)}>↗</span>
                                    </>
                                )}
                            </span>
                        ),
                    )}
                </Fade>
            </nav>
            <div {...style.attrs(appStyles.main)}>
                <article data-component="Page" {...style.attrs(appStyles.page)}>
                    <div data-component="PageHeader" {...style.attrs(appStyles.toolbar)}>
                        <b data-component="PageTitle" {...style.attrs(appText.title)}>
                            Launch plan
                        </b>
                        <div {...style.attrs(appStyles.tools)}>
                            <Fade
                                isOpen={properties.isOpenAt(11)}
                                xstyle={styles.roomy}
                                component={
                                    properties.isOpenAt(11) ? "PreviewButton" : "VercelPreviewLink"
                                }
                            >
                                <span data-service="11" {...style.attrs(appStyles.action)}>
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
                                    {...style.attrs(
                                        appStyles.action,
                                        properties.isOpenAt(12) && appStyles.actionStrong,
                                    )}
                                >
                                    {properties.isOpenAt(12) ? "Edit app" : "Send feedback"}
                                </span>
                            </Fade>
                            <span
                                data-component="ShareButton"
                                data-service="2"
                                {...style.attrs(appStyles.primary)}
                            >
                                Share
                            </span>
                        </div>
                    </div>
                    <div data-component="PropertyList" {...style.attrs(styles.properties)}>
                        <span data-component="StatusProperty" {...style.attrs(appStyles.property)}>
                            <span {...style.attrs(styles.state, styles.started, styles.flush)} />
                            In progress
                        </span>
                        <span data-component="DateProperty" {...style.attrs(appStyles.property)}>
                            Fri 24 Oct
                        </span>
                        <Fade
                            isOpen={properties.isOpenAt(3)}
                            component={properties.isOpenAt(3) ? "PriceProperty" : "StripeKeyPrompt"}
                            xstyle={styles.slot}
                        >
                            <span
                                data-service="3"
                                {...style.attrs(
                                    appStyles.property,
                                    !properties.isOpenAt(3) && styles.propertyMissing,
                                )}
                            >
                                <span
                                    style={{ "mask-image": "url(/diagram/vault.svg)" }}
                                    {...style.attrs(appStyles.glyph)}
                                />
                                {properties.isOpenAt(3)
                                    ? "Pro €12/mo · live"
                                    : "Paste a Stripe API key to show prices"}
                            </span>
                        </Fade>
                        <span
                            data-component="ScheduleProperty"
                            data-service="8"
                            {...style.attrs(appStyles.property)}
                        >
                            Reminder Thu
                        </span>
                    </div>
                    <p data-component="Paragraph" {...style.attrs(styles.text)}>
                        Ship to the waitlist on Thursday.{" "}
                        <span data-component="Presence">
                            Pricing stays in step with billing
                            <span data-component="Cursor" {...style.attrs(styles.cursor)}>
                                <span {...style.attrs(styles.flag)}>Agent</span>
                            </span>
                        </span>
                    </p>
                    <Fade
                        isOpen={properties.isOpenAt(8)}
                        component={properties.isOpenAt(8) ? "AgentNote" : "AutomationError"}
                    >
                        <p
                            data-service="8"
                            {...style.attrs(
                                styles.agentNote,
                                !properties.isOpenAt(8) && styles.error,
                            )}
                        >
                            {properties.isOpenAt(8) ? (
                                <>
                                    <Tile name="agent" />
                                    <span>
                                        <b {...style.attrs(styles.strong)}>Agent</b>{" "}
                                        <span {...style.attrs(styles.quiet)}>
                                            drafted LCH-15 for you to review
                                        </span>
                                    </span>
                                </>
                            ) : (
                                <>
                                    <img
                                        alt=""
                                        src="/logos/temporal.png"
                                        {...style.attrs(styles.logo)}
                                    />
                                    <span>
                                        <b {...style.attrs(styles.strong)}>Run failed</b>{" "}
                                        <span>Draft the launch post · retrying in 5 min</span>
                                    </span>
                                </>
                            )}
                        </p>
                    </Fade>
                    <Fade
                        isOpen={properties.isOpenAt(4)}
                        component={properties.isOpenAt(4) ? "TaskTable" : "LinearEmbed"}
                        xstyle={properties.isOpenAt(4) ? styles.tasks : styles.embed}
                        isLarge
                    >
                        <p data-service="4" {...style.attrs(styles.tableBar)}>
                            {properties.isOpenAt(4) ? (
                                <>
                                    <Tile name="tasks" />
                                    <b {...style.attrs(styles.strong)}>Launch tasks</b>
                                    <span {...style.attrs(styles.quiet, styles.wide)}>
                                        from Tasks, also in My week
                                    </span>
                                    <span {...style.attrs(appStyles.tools)}>
                                        <Access isOpen={true} />
                                        <span {...style.attrs(styles.newTask)}>+ New task</span>
                                    </span>
                                </>
                            ) : (
                                <>
                                    <Tile name="linear" />
                                    <b {...style.attrs(styles.strong)}>Linear embed</b>
                                    <span {...style.attrs(styles.quiet, styles.wide)}>
                                        Read-only · synced 3 h ago
                                    </span>
                                    <span {...style.attrs(appStyles.tools)}>
                                        <Access isOpen={false} />
                                        <span {...style.attrs(styles.failing)}>Reconnect</span>
                                    </span>
                                </>
                            )}
                        </p>
                        <table
                            data-service="4"
                            {...style.attrs(styles.table, !properties.isOpenAt(4) && styles.stale)}
                        >
                            <thead>
                                <tr>
                                    <th {...style.attrs(styles.head, styles.id)}>ID</th>
                                    <th {...style.attrs(styles.head)}>Task</th>
                                    <th {...style.attrs(styles.head, styles.wide)}>Owner</th>
                                    <th {...style.attrs(styles.head, styles.wide)}>Due</th>
                                    <th {...style.attrs(styles.head, styles.wide)}>
                                        {properties.isOpenAt(12) ? "Checks" : "Pull request"}
                                    </th>
                                    {properties.isOpenAt(4) && properties.isOpenAt(12) ? (
                                        <th
                                            data-component="AddedColumn"
                                            data-service="12"
                                            {...style.attrs(styles.head, styles.added, styles.wide)}
                                        >
                                            Blocked by
                                            <span {...style.attrs(styles.yours)}>yours</span>
                                        </th>
                                    ) : undefined}
                                </tr>
                            </thead>
                            <tbody>
                                {tasks.map(([id, title, status, assignee, tint, due, change]) => (
                                    <tr data-component="ObjectRow">
                                        <td {...style.attrs(styles.cell, styles.id)}>{id}</td>
                                        <td {...style.attrs(styles.cell, styles.taskTitle)}>
                                            <span {...style.attrs(styles.state, styles[status])} />
                                            {title}
                                        </td>
                                        <td {...style.attrs(styles.cell, styles.wide)}>
                                            <span {...style.attrs(styles.row)}>
                                                <span
                                                    data-component="Avatar"
                                                    {...style.attrs(appStyles.avatar)}
                                                    style={{ "background-color": tint }}
                                                >
                                                    {assignee === "Me" ? "F" : assignee.charAt(0)}
                                                </span>
                                            </span>
                                        </td>
                                        <td
                                            {...style.attrs(styles.cell, styles.quiet, styles.wide)}
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
                                            {...style.attrs(
                                                styles.cell,
                                                styles.change,
                                                styles.wide,
                                            )}
                                        >
                                            {change === undefined ? undefined : (
                                                <span {...style.attrs(styles.row)}>
                                                    <span
                                                        style={{
                                                            "mask-image": `url(/diagram/${properties.isOpenAt(12) ? "source" : "github"}.svg)`,
                                                        }}
                                                        {...style.attrs(appStyles.glyph)}
                                                    />
                                                    {properties.isOpenAt(12)
                                                        ? undefined
                                                        : `${change[0]} ↗`}
                                                    <span
                                                        {...style.attrs(
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
                                                {...style.attrs(
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
                    <b data-component="Heading" {...style.attrs(styles.heading)}>
                        Notes from Monday
                    </b>
                    <p data-component="Paragraph" {...style.attrs(styles.text)}>
                        The waitlist has 1,840 people. Your agent sends the first batch of invites
                        on Thursday morning, and the rest follow once the demo is live.
                    </p>
                    <p data-component="Attachment" data-service="5" {...style.attrs(styles.file)}>
                        <span
                            style={{ "mask-image": "url(/diagram/file.svg)" }}
                            {...style.attrs(appStyles.glyph)}
                        />
                        pricing-v4.pdf
                        <span {...style.attrs(styles.quiet)}>
                            {properties.isOpenAt(5)
                                ? "1.2 MB · added by you"
                                : "Dropbox ↗ · request access"}
                        </span>
                    </p>
                    <p data-component="Comment" {...style.attrs(styles.comment)}>
                        <span
                            data-component="Avatar"
                            {...style.attrs(appStyles.avatar)}
                            style={{ "background-color": "#6b5ca5" }}
                        >
                            A
                        </span>
                        <span>
                            <b {...style.attrs(styles.strong)}>Agent</b>{" "}
                            <span {...style.attrs(styles.quiet)}>
                                The launch post draft is ready for you
                            </span>
                        </span>
                    </p>
                    <p data-component="Paragraph" {...style.attrs(styles.text)}>
                        Pricing launches with three plans. The team plan replaces the old seat
                        pricing, and existing customers keep their rate for a year.
                    </p>
                    <b data-component="Heading" {...style.attrs(styles.heading)}>
                        Before launch
                    </b>
                    <ul data-component="Checklist" {...style.attrs(styles.checklist)}>
                        {checklist.map(([item, isDone]) => (
                            <li data-component="CheckItem" {...style.attrs(styles.check)}>
                                <span {...style.attrs(styles.box, isDone && styles.boxDone)} />
                                <span {...style.attrs(isDone && styles.struck)}>{item}</span>
                            </li>
                        ))}
                    </ul>
                </article>
            </div>
            <div data-component="StatusBar" {...style.attrs(appStyles.statusBar)}>
                <span
                    data-component="SyncStatus"
                    data-service="11"
                    {...style.attrs(appStyles.status)}
                >
                    <span
                        {...style.attrs(appStyles.dot, !properties.isOpenAt(11) && styles.dotCloud)}
                    />
                    {properties.isOpenAt(11) ? "Yours · on your laptop" : "Saved to their cloud"}
                </span>
                <span
                    data-component={properties.isOpenAt(4) ? "PresenceList" : "ConflictNotice"}
                    data-service="4"
                    {...style.attrs(appStyles.status, styles.wide)}
                >
                    <span
                        {...style.attrs(appStyles.avatar, styles.small)}
                        style={{ "background-color": "#2f7d8c" }}
                    >
                        F
                    </span>
                    <span
                        {...style.attrs(appStyles.avatar, styles.small)}
                        style={{ "background-color": "#6b5ca5" }}
                    >
                        A
                    </span>
                    {properties.isOpenAt(4) ? (
                        <span {...style.attrs(styles.presenceText)}>
                            You and your agent editing
                        </span>
                    ) : (
                        <span {...style.attrs(styles.presenceText)}>
                            Agent's edits
                            <span {...style.attrs(styles.reload)}>Reload</span>
                        </span>
                    )}
                </span>
                <span {...style.attrs(appStyles.statusEnd)}>
                    <span
                        data-component={properties.isOpenAt(9) ? "ViewCount" : "AnalyticsSnippet"}
                        data-service="9"
                        {...style.attrs(appStyles.status, styles.wide)}
                    >
                        {properties.isOpenAt(9) ? "182 views" : "PostHog ↗"}
                    </span>
                    <span
                        data-component="ErrorCount"
                        data-service="10"
                        {...style.attrs(
                            appStyles.status,
                            !properties.isOpenAt(10) && styles.failing,
                        )}
                    >
                        {properties.isOpenAt(10) ? "0 errors" : "3 integrations failing"}
                    </span>
                    <span
                        data-component="BranchMenu"
                        data-service="12"
                        {...style.attrs(appStyles.status, styles.wide)}
                    >
                        <span
                            style={{ "mask-image": "url(/diagram/source.svg)" }}
                            {...style.attrs(appStyles.glyph)}
                        />
                        main
                    </span>
                </span>
            </div>
        </div>
    );
}

/** The media query for phones, where wide-only parts hide over whichever display their base sets. */
const phone = "@media (max-width: 767px)";

/** The Pages app styles. */
const styles = style.create({
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
        borderColor: color.border,
        borderRadius: "8px",
        overflow: "hidden",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "grid",
    },
    embed: {
        borderColor: color.border,
        borderRadius: "8px",
        overflow: "hidden",
        borderStyle: "dashed",
        borderWidth: stroke.border,
        display: "grid",
    },
    tableBar: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "flex",
        gap: "0.5rem",
        margin: 0,
        paddingBlock: "0.5rem",
        paddingInline: "0.75rem",
        whiteSpace: "nowrap",
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
    railApps: {
        display: "grid",
        flexShrink: 0,
        gap: "0.5rem",
    },
    head: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
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
        fontFamily: font.code,
        fontSize: "0.58rem",
        fontWeight: 400,
        letterSpacing: "0.06em",
        marginLeft: "0.375rem",
        textTransform: "uppercase",
    },
    roomy: {
        display: { default: "block", [media.maxMd]: "none" },
    },
    slot: {
        display: "flex",
    },
    presenceText: {
        marginLeft: "0.375rem",
    },
    reload: {
        color: color.foreground,
        fontWeight: 600,
        marginLeft: "0.5rem",
    },
    access: {
        alignItems: "center",
        color: color.mutedForeground,
        display: { default: "flex", [media.maxMd]: "none" },
        gap: "0.375rem",
    },
    change: {
        fontSize: "0.75rem",
    },
    passing: {
        color: palette.green,
        fontWeight: 600,
    },
    running: {
        color: palette.ochre,
        fontWeight: 600,
    },
    newTask: {
        borderColor: color.border,
        borderRadius: "5px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        color: color.foreground,
        fontWeight: 600,
        paddingBlock: "0.125rem",
        paddingInline: "0.5rem",
    },
    failing: {
        color: palette.red,
        fontWeight: 600,
    },
    stale: {
        opacity: 0.55,
    },
    error: {
        backgroundColor: `color-mix(in srgb, ${palette.red} 9%, transparent)`,
        color: palette.brick,
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
        borderWidth: stroke.border,
    },
    dotCloud: {
        backgroundColor: color.mutedForeground,
    },
    row: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
    },
    heading: {
        fontSize: "1rem",
        fontWeight: 700,
        marginTop: "0.25rem",
    },
    file: {
        alignItems: "center",
        alignSelf: "start",
        borderColor: color.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: stroke.border,
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
        backgroundColor: palette.indigo,
        borderColor: palette.indigo,
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
    properties: {
        alignItems: "center",
        display: "flex",
        flexWrap: "wrap",
        gap: "0.5rem",
        margin: 0,
    },
    text: {
        lineHeight: 1.6,
        margin: 0,
    },
    cursor: {
        backgroundColor: palette.violet,
        display: "inline-block",
        height: "1.15em",
        marginLeft: "1px",
        position: "relative",
        verticalAlign: "-0.2em",
        width: "2px",
    },
    flag: {
        backgroundColor: palette.violet,
        borderStartStartRadius: "3px",
        borderStartEndRadius: "3px",
        borderEndEndRadius: "3px",
        borderEndStartRadius: "0",
        bottom: "100%",
        color: "white",
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
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        paddingBlock: "0.625rem",
        paddingRight: "0.75rem",
        whiteSpace: "nowrap",
        ":last-child": { paddingRight: "0.75rem" },
    },
    id: {
        color: color.mutedForeground,
        fontFamily: font.code,
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
        backgroundImage: `linear-gradient(90deg, ${palette.amber} 50%, transparent 50%)`,
        borderColor: palette.amber,
    },
    flush: {
        marginRight: 0,
    },
    done: {
        backgroundColor: palette.indigo,
        borderColor: palette.indigo,
    },
    strong: {
        fontWeight: 600,
    },
    quiet: {
        color: color.mutedForeground,
    },
    wide: { [phone]: { display: "none" } },
    small: {
        fontSize: "0.5rem",
        height: "0.9375rem",
        marginRight: "-0.25rem",
        width: "0.9375rem",
    },
});
