import type { Session } from "@destack/machine/device";
import { type ObjectClient, RelationalQuery } from "@destack/object/client";
import { type Message, message } from "@destack/message/client";
import { activity, type Activity, Inbox, notification } from "@destack/notification/client";
import { type Identifier, present } from "@destack/schema";
import { installation } from "@destack/object";
import type { BannerAction, Command } from "../shell/index.ts";
import { DesktopError } from "../error/index.ts";

/** The message statuses a desktop follows: sent to it, and delivered on it. */
const FOLLOWED_STATUSES: Message["status"][] = ["sent", "delivered"];

/** What the system's banners offer on a platform: actions and withdrawal only on macOS. */
export class BannerCapabilities {
    /** Whether a shown banner can be withdrawn. */
    readonly isWithdrawable: boolean;
    /** Whether a banner can offer actions. */
    readonly isActionable: boolean;

    /** Take what a platform's banners offer. */
    constructor(fields: Pick<BannerCapabilities, "isWithdrawable" | "isActionable">) {
        this.isWithdrawable = fields.isWithdrawable;
        this.isActionable = fields.isActionable;
    }

    /** Read what banners offer on a platform. */
    static of(platform: NodeJS.Platform): BannerCapabilities {
        const isApple = platform === "darwin";

        return new BannerCapabilities({ isWithdrawable: isApple, isActionable: isApple });
    }
}

/** One system banner: a desktop message to this client, telling of an unread activity. */
export class Banner {
    /** The message. */
    readonly id: string;
    /** Whether its message was delivered on this desktop already. */
    readonly isDelivered: boolean;
    /** The activity the banner tells of. */
    readonly activity: Pick<
        Activity,
        "id" | "scope" | "packageId" | "content" | "actions" | "interruption"
    >;

    /** Take a message as its banner. */
    constructor(fields: Pick<Banner, "id" | "isDelivered" | "activity">) {
        this.id = fields.id;
        this.isDelivered = fields.isDelivered;
        this.activity = fields.activity;
    }

    /** Follow the messages a desktop shows and the inbox they tell of, read together. */
    static follow(
        inbox: ObjectClient<{
            readonly notification: typeof notification;
            readonly activity: typeof activity;
            readonly message: typeof message;
        }>,
        signal: AbortSignal,
    ) {
        const { query } = inbox;

        return RelationalQuery.latest(
            {
                messages: query.message.findMany({ where: { status: { in: FOLLOWED_STATUSES } } }),
                notifications: query.notification.findMany({ with: { activity: true } }),
            },
            signal,
        );
    }

    /** Select a client's messages whose notification is still unread in the inbox, as banners. */
    static select(
        messages: readonly Pick<Message, "id" | "to" | "sourceId" | "status">[],
        inbox: Inbox,
        client: Identifier<"client">,
    ): Banner[] {
        // index the inbox's unread entries by their notification
        const unread = new Map(
            inbox.entries
                .filter((entry) => entry.isUnread && !entry.isArchived)
                .map((entry) => [String(entry.notification.id), entry]),
        );

        // take the messages to this client whose notification is unread
        return messages.flatMap((row): Banner[] => {
            const entry = unread.get(row.sourceId);
            const isOwn = row.to.channel === "desktop" && row.to.client === client;

            return isOwn && entry !== undefined
                ? [
                      new Banner({
                          id: row.id,
                          isDelivered: row.status === "delivered",
                          activity: entry.activity,
                      }),
                  ]
                : [];
        });
    }

    /** Plan the shell commands that make the shown banners match the selected ones. */
    static plan(
        shown: ReadonlyMap<string, Banner>,
        banners: readonly Banner[],
        capabilities: BannerCapabilities,
    ): Command[] {
        // withdraw the banners gone, on platforms whose banners withdraw
        const ids = new Set<string>(banners.map((banner) => banner.id));
        const withdrawn = capabilities.isWithdrawable
            ? [...shown.keys()]
                  .filter((id) => !ids.has(id))
                  .map((id): Command => ({ action: "banner.withdraw", id }))
            : [];

        // show each new banner the desktop never showed
        const shows = banners.flatMap((banner): Command[] => {
            if (shown.has(banner.id) || banner.isDelivered) {
                return [];
            }
            const { title, subtitle, body } = banner.activity.content;

            return [
                {
                    action: "banner.show",
                    id: banner.id,
                    title,
                    ...(subtitle === undefined ? {} : { subtitle }),
                    body,
                    interruption: banner.activity.interruption,
                    actions: capabilities.isActionable
                        ? Banner.#actions(banner.activity.actions)
                        : [],
                },
            ];
        });

        return [...withdrawn, ...shows];
    }

    /** Answer the banner's action as the person on the activity it tells of. */
    async act(
        session: Pick<Session, "address" | "open">,
        name: string,
        text?: string,
    ): Promise<void> {
        // find the installation declaring the notification in the activity's space
        const shown = this.activity;
        const address = await session.address(shown.scope);
        await using stack = new AsyncDisposableStack();
        const installations = await session.open({ space: address }, { installation }, stack);
        const declaring = await installations.query.installation.findFirst({
            where: { packageId: shown.packageId },
        });
        if (declaring === undefined) {
            throw DesktopError.of(
                "INSTALLATION_NOT_FOUND",
                `${address} has no installation of ${shown.packageId}`,
            );
        }

        // run the action there as the person
        const activities = await session.open(
            {
                space: address,
                installation: present(declaring.name, `the name of installation ${declaring.id}`),
            },
            { activity },
            stack,
        );
        const submission = activities.mutate(activity).act({
            id: shown.id,
            action: name,
            ...(text === undefined ? {} : { text }),
        });
        await submission.confirmed;
    }

    /** List an activity's actions as a banner offers them. */
    static #actions(actions: Activity["actions"]): BannerAction[] {
        return actions.map((action) => ({
            name: action.name,
            title: action.title,
            isDestructive: action.isDestructive === true,
            ...(action.text === undefined ? {} : { text: action.text }),
        }));
    }
}
