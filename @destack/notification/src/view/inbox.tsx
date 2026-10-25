import { plural, t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { text } from "@destack/theme/text";
import { color, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { Badge } from "@destack/ui/badge";
import {
    createMemo,
    For,
    type JSX,
    omit,
    Show,
    useHome,
    useLocale,
    useNow,
    useQuery,
} from "@destack/view";
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
    readonly inbox: () => Inbox | undefined;
    readonly now: () => number | undefined;
} {
    // follow the home's unarchived notifications and activity replicas at the view's time
    const home = useHome({ notification, activity });
    const now = useNow();
    const notifications = useQuery(() =>
        home.query.notification.findMany({ where: { archivedAt: { isNull: true } } }),
    );
    const activities = useQuery(() => home.query.activity.findMany({}));

    return {
        read: (entry) => home.mutate.notification.read({ id: entry.notification.id }),
        inbox: createMemo(() => {
            // decide snoozes once mounted, when the view knows the time
            const time = now();

            return time === undefined ? undefined : new Inbox(notifications(), activities(), time);
        }),
        now,
    };
}

/** Render the person's notifications latest first, marking one read when opened. */
export function NotificationInbox(properties: NotificationInboxProperties): JSX.Element {
    // follow the inbox's active notifications, latest first, and the view's time
    const locale = useLocale();
    const rest = omit(properties, "onOpen", "style");
    const { read, inbox, now } = useInbox();

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
            {/* Entries, once the view knows the time */}
            <Show when={inbox()}>
                {(current) => (
                    <For
                        each={current().active()}
                        keyed={(entry) => entry.notification.id}
                        fallback={
                            <li {...style.attrs(styles.empty)}>
                                {locale.render(t`No notifications`)}
                            </li>
                        }
                    >
                        {(entry) => <NotificationRow entry={entry()} now={now} onOpen={open} />}
                    </For>
                )}
            </Show>
        </ul>
    );
}

/** Render one notification as a button showing its line, its time and whether it is unread. */
function NotificationRow(properties: {
    readonly entry: InboxEntry;
    readonly now: () => number | undefined;
    readonly onOpen: (entry: InboxEntry) => void;
}): JSX.Element {
    // read the locale and whether the entry is unread
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
                        <Show when={properties.now()}>
                            {(time) =>
                                locale.relative(properties.entry.activity.occurredAt, time())
                            }
                        </Show>
                    </time>
                </span>
            </button>
        </li>
    );
}

/** Render the person's unread notifications as a count, for a space and app or all of them. */
export function NotificationBadge(properties: NotificationBadgeProperties): JSX.Element {
    // count the unread notifications the badge covers, showing none for no count or zero
    const locale = useLocale();
    const { inbox } = useInbox();
    const unread = (): number | undefined =>
        inbox()?.unread({
            ...(properties.space === undefined ? {} : { space: properties.space }),
            ...(properties.packageId === undefined ? {} : { packageId: properties.packageId }),
        });

    return (
        <Show when={unread()}>
            {(count) => (
                <Badge data-slot="notification-badge">
                    {/* Count */}
                    <span aria-hidden="true">{locale.number(count())}</span>

                    {/* Spoken count */}
                    <span {...style.attrs(styles.hidden)}>
                        {locale.render(
                            t`${plural(count(), { one: "# unread notification", other: "# unread notifications" })}`,
                        )}
                    </span>
                </Badge>
            )}
        </Show>
    );
}
