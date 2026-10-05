import { plural, t } from "@destack/locale";
import type { Identifier } from "@destack/schema";
import * as style from "@destack/style";
import { color, radius, space, stroke, text, weight } from "@destack/theme/tokens.stylex";
import { Badge } from "@destack/ui/badge";
import { useLocale } from "@destack/locale/solid";
import { For, omit, Show, useHome, useQuery, type JSX } from "@destack/view";
import { NOTIFICATIONS, UNREAD } from "../inbox/inbox.ts";
import { notification } from "../object/activity.ts";

/** The styles of an inbox and its entries. */
const styles = style.create({
    inbox: {
        display: "flex",
        flexDirection: "column",
        margin: 0,
        padding: 0,
        listStyle: "none",
        fontFamily: text.footnoteFontFamily,
        fontSize: text.footnoteFontSize,
        lineHeight: text.footnoteLineHeight,
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
        backgroundColor: { default: "transparent", ":hover": color.accent },
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
        fontFamily: text.captionFontFamily,
        fontSize: text.captionFontSize,
        fontWeight: weight.regular,
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

/** A notification as the inbox shows it. */
export interface NotificationEntry {
    /** The notification's id. */
    readonly id: Identifier<"notification">;
    /** The line it shows, rendered in the person's locale where it happened. */
    readonly summary: string;
    /** When it last occurred, in epoch milliseconds. */
    readonly occurredAt: number;
    /** When the person read it, null while unread. */
    readonly readAt: number | null;
}

/** The properties of a notification inbox, the native list's attributes included. */
export interface NotificationInboxProperties extends Omit<
    JSX.HTMLAttributes<HTMLUListElement>,
    "class" | "style"
> {
    /** Handle a notification being opened, after it is marked read. */
    readonly onOpen?: (entry: NotificationEntry) => void;
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

/** Render the person's notifications latest first, marking one read when opened. */
export function NotificationInbox(properties: NotificationInboxProperties): JSX.Element {
    // follow the inbox, latest first
    const locale = useLocale();
    const rest = omit(properties, "onOpen", "style");
    const home = useHome({ notification });
    const entries = useQuery(() => home.query.notification.findMany(NOTIFICATIONS));

    // mark a notification read and hand it to the owner
    const open = (entry: NotificationEntry) => {
        if (entry.readAt === null) {
            home.mutate.notification.read({ id: entry.id });
        }
        properties.onOpen?.(entry);
    };

    return (
        <ul
            aria-label={locale.render(t`Notifications`)}
            data-slot="notification-inbox"
            {...rest}
            {...style.attrs(styles.inbox, properties.style)}
        >
            <For
                each={entries()}
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
    readonly entry: NotificationEntry;
    readonly onOpen: (entry: NotificationEntry) => void;
}): JSX.Element {
    const locale = useLocale();
    const isUnread = (): boolean => properties.entry.readAt === null;

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
                    {properties.entry.summary}
                    <time
                        datetime={new Date(properties.entry.occurredAt).toISOString()}
                        {...style.attrs(styles.time)}
                    >
                        {locale.relative(properties.entry.occurredAt, Date.now())}
                    </time>
                </span>
            </button>
        </li>
    );
}

/** Render the person's unread notifications as a count, for a space and app or all of them, hidden at none. */
export function NotificationBadge(properties: NotificationBadgeProperties): JSX.Element {
    // sum the unread counts of the groups the badge covers
    const locale = useLocale();
    const home = useHome({ notification });
    const groups = useQuery(() => home.query.notification.aggregate(UNREAD));
    const unread = (): number =>
        groups()
            .filter((count) => (properties.space ?? count.group.space) === count.group.space)
            .filter(
                (count) =>
                    (properties.packageId ?? count.group.packageId) === count.group.packageId,
            )
            .reduce((sum, count) => sum + count.values.unread, 0);

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
