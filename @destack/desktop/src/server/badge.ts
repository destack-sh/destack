import {
    type activity,
    Inbox,
    type InboxRow,
    type notification,
} from "@destack/notification/client";
import type { message } from "@destack/message/client";
import type { ObjectClient } from "@destack/object/client";
import type { Shell } from "../shell/index.ts";

/** The application icon's badge: the unread count of the person's inbox. */
export class Badge {
    /** The native shell showing the badge. */
    readonly #shell: Shell;

    /** Show the badge through the shell. */
    constructor(shell: Shell) {
        this.#shell = shell;
    }

    /** Count an inbox's unread notifications of every space and app. */
    static count(rows: readonly InboxRow[], now: number): number {
        return new Inbox(rows, now).unread();
    }

    /** Show the inbox's unread count as it changes, until aborted or its client fails. */
    async follow(
        inbox: ObjectClient<{
            readonly notification: typeof notification;
            readonly activity: typeof activity;
            readonly message: typeof message;
        }>,
        signal: AbortSignal,
    ): Promise<void> {
        // count the unread notifications at each change of the inbox
        const { clock } = inbox.environment;
        const rows = inbox.query.notification.findMany({ with: { activity: true } }).watch(signal);
        for await (const read of rows) {
            await this.show(Badge.count(read, clock.now()));
        }
    }

    /** Show a count, clearing the badge at zero. */
    async show(count: number): Promise<void> {
        await this.#shell.send({ action: "badge.set", count: count === 0 ? null : count });
    }

    /** Clear the badge, as nobody is signed in to read it. */
    async clear(): Promise<void> {
        await this.show(0);
    }
}
