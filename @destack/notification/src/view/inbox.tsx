import { plural, t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { text } from "@destack/theme/text";
import { color, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { Badge } from "@destack/ui/badge";
import { For, type JSX, omit, Show, useHome, useLocale, useQuery } from "@destack/view";
import { Inbox, type InboxEntry } from "../notification/inbox.ts";
import { activity } from "../object/activity.ts";
import { notification } from "../object/notification.ts";

/** The styles of an inbox and its entries. */
const styles = style.create({
    inbox: {
        display: "flex",
        flexDirection: "column",
        margin: 0,
        padding: 0,
        listStyle: "none",
    },
    entry: {
        display: "flex",
        alignItems: "flex-start",
        gap: space[2],
        width: "100%",
        padding: space[3],
        borderWidth: 0,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        borderBottomColor: color.border,
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.accent },
        },
        color: "inherit",
        textAlign: "start",
        cursor: "pointer",
        fontFamily: "inherit",
        fontSize: "inherit",
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
    unread: {
        fontWeight: weight.medium,
    },
    dot: {
        flexShrink: 0,
        width: space[2],
        height: space[2],
        marginTop: space[1],
        borderRadius: radius.full,
    },
    dotUnread: {
        backgroundColor: color.primary,
    },
    body: {
        display: "flex",
        flex: 1,
        flexDirection: "column",
        gap: space[1],
    },
    time: {
        color: color.mutedForeground,
    },
    empty: {
        padding: space[5],
        color: color.mutedForeground,
        textAlign: "center",
    },
    hidden: {
        position: "absolute",
        width: stroke.border,
        height: stroke.border,
        overflow: "hidden",
        clipPath: "inset(50%)",
        whiteSpace: "nowrap",
    },
});

/** The properties of a notification inbox, the native list's attributes included. */
export interface NotificationInboxProperties extends Omit<
    JSX.HTMLAttributes<HTMLUListElement>,
    "class" | "style"
> {
    /** Handle a notification being opened, after it is marked read. */
    readonly onOpen?: (entry: InboxEntry) => void;
    /** The StyleX styles applied after the inbox's styles. */
    readonly style?: style.Styles;
}

/** The properties of a notification badge. */
export interface NotificationBadgeProperties {
    /** The space whose unread notifications to count, every space by default. */
    readonly space?: string;
    /** The app whose unread notifications to count, every app by default. */
    readonly packageId?: string;
}

/** Follow the person's inbox: their notifications joined with their activity replicas. */
function useInbox(): {
    readonly read: (entry: InboxEntry) => void;
    readonly inbox: () => Inbox;
} {
    // follow the home's notifications and activity replicas
    const home = useHome({ notification, activity });
    const notifications = useQuery(() => home.query.notification.findMany({}));
    const activities = useQuery(() => home.query.activity.findMany({}));

    return {
        read: (entry) => home.mutate.notification.read({ id: entry.notification.id }),
        inbox: () => new Inbox(notifications(), activities(), Date.now()),
    };
}

/** Render the person's notifications latest first, marking one read when opened. */
export function NotificationInbox(properties: NotificationInboxProperties): JSX.Element {
    // follow the inbox's notifications neither archived nor snoozed, latest first
    const locale = useLocale();
    const rest = omit(properties, "onOpen", "style");
    const { read, inbox } = useInbox();
    const listed = () => inbox().entries.filter((entry) => !entry.isArchived && !entry.isSnoozed);

    // mark a notification read and hand it to the owner
    const open = (entry: InboxEntry) => {
        if (entry.isUnread) {
            read(entry);
        }
        properties.onOpen?.(entry);
    };

    return (
        <ul
            aria-label={locale.render(t`Notifications`)}
            data-slot="notification-inbox"
            {...rest}
            {...style.attrs(text.footnote, styles.inbox, properties.style)}
        >
            <For
                each={listed()}
                fallback={
                    <li {...style.attrs(styles.empty)}>{locale.render(t`No notifications`)}</li>
                }
            >
                {(entry) => <NotificationRow entry={entry} onOpen={open} />}
            </For>
        </ul>
    );
}

/** Render one notification as a button showing its line, its time and whether it is unread. */
function NotificationRow(properties: {
    readonly entry: InboxEntry;
    readonly onOpen: (entry: InboxEntry) => void;
}): JSX.Element {
    const locale = useLocale();
    const isUnread = (): boolean => properties.entry.isUnread;

    return (
        <li data-slot="notification" data-state={isUnread() ? "unread" : "read"}>
            <button
                type="button"
                onClick={() => properties.onOpen(properties.entry)}
                {...style.attrs(styles.entry, isUnread() && styles.unread)}
            >
                <span
                    aria-hidden="true"
                    {...style.attrs(styles.dot, isUnread() && styles.dotUnread)}
                />
                <span {...style.attrs(styles.body)}>
                    <Show when={isUnread()}>
                        <span {...style.attrs(styles.hidden)}>{locale.render(t`Unread`)} </span>
                    </Show>
                    {properties.entry.activity.summary}
                    <time
                        datetime={new Date(properties.entry.activity.occurredAt).toISOString()}
                        {...style.attrs(text.caption, styles.time)}
                    >
                        {locale.relative(properties.entry.activity.occurredAt, Date.now())}
                    </time>
                </span>
            </button>
        </li>
    );
}

/** Render the person's unread notifications as a count, for a space and app or all of them. */
export function NotificationBadge(properties: NotificationBadgeProperties): JSX.Element {
    // count the unread notifications the badge covers
    const locale = useLocale();
    const { inbox } = useInbox();
    const unread = (): number =>
        inbox().unread({
            ...(properties.space === undefined ? {} : { space: properties.space }),
            ...(properties.packageId === undefined ? {} : { packageId: properties.packageId }),
        });

    return (
        <Show when={unread() > 0}>
            <Badge data-slot="notification-badge">
                {/* Count */}
                <span aria-hidden="true">{locale.number(unread())}</span>

                {/* Spoken count */}
                <span {...style.attrs(styles.hidden)}>
                    {locale.render(
                        t`${plural(unread(), { one: "# unread notification", other: "# unread notifications" })}`,
                    )}
                </span>
            </Badge>
        </Show>
    );
}
