import { type Select, TABLE } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { subjectContext } from "@destack/service/test";
import type { Page, Subject } from "@destack/sync";
import { UNREAD } from "../inbox/index.ts";
import { notification } from "../object/index.ts";

/** A row of a change, read for its identifier. */
const RowIdentity = schema.looseObject({ id: schema.string() });

/** An unread group of the inbox: its space and app. */
const BadgeGroup = schema.object({ space: schema.string(), packageId: PackageId });

/** An unread group's counts. */
const BadgeValues = schema.object({ unread: schema.number(), latestAt: schema.number() });

/** The unread notifications of one app in one space. */
export interface Badge {
    /** The space of the notifications. */
    readonly space: string;
    /** The app. */
    readonly packageId: PackageId;
    /** The unread count. */
    readonly unread: number;
    /** When the latest occurred. */
    readonly latestAt: number;
}

/** A person's inbox in their home: its notifications and their live unread counts. */
export class HomeInbox {
    /** The rows kept, by identifier. */
    readonly rows = new Map<string, Select<typeof notification.table>>();
    /** The unread counts kept, by space and app. */
    readonly counts = new Map<string, Badge>();
    /** End the stream of pages. */
    readonly #following = new AbortController();
    /** The reading of every page, settled once the stream ends. */
    readonly #reading: Promise<void>;
    /** The pages read so far. */
    #pages = 0;
    /** Wake whoever waits for the next page. */
    #wake: () => void = () => {};

    /** Follow a home's notifications as a person until closed. */
    constructor(store: Pick<ObjectStore, "source">, homeId: string, person: Subject) {
        // read every page in the background into the rows and counts kept
        const context = subjectContext(person, homeId, { signal: this.#following.signal });
        const { where, ...aggregate } = UNREAD;
        const pages = store.source.relayed(
            store.source.queriesShape.subscription({
                name: "objects",
                scope: homeId,
                below: homeId,
                parameters: {
                    queries: {
                        notifications: { object: "notification" },
                        unread: { object: "notification", where, aggregate },
                    },
                },
            }),
            context,
        );
        this.#reading = (async () => {
            for await (const page of pages) {
                this.#apply(page);
                this.#pages += 1;
                this.#wake();
            }
        })();
    }

    /** Read the unread count across spaces and apps, as a dock badge shows it. */
    get unread(): number {
        return [...this.counts.values()].reduce((total, badge) => total + badge.unread, 0);
    }

    /** End the stream and wait for its last page. */
    async close(): Promise<void> {
        this.#following.abort();
        await this.#reading;
    }

    /** Wait until the kept rows and counts meet a condition, reading each arriving page. */
    async until(condition: (inbox: HomeInbox) => boolean): Promise<void> {
        while (!condition(this)) {
            await this.#next(this.#pages + 1);
        }
    }

    /** Wait for a page. */
    async #next(page: number): Promise<void> {
        while (this.#pages < page) {
            await new Promise<void>((resolve) => {
                this.#wake = resolve;
            });
        }
    }

    /** Apply a page's rows and aggregate groups. */
    #apply(page: Page): void {
        // start over from a snapshot that resets
        if (page.reset) {
            this.rows.clear();
            this.counts.clear();
        }

        // keep the notification rows entering and changing, and drop those leaving
        for (const change of page.changes) {
            const { id } = RowIdentity.parse(change.row);
            // drop a row leaving
            if (change.operation === "delete") {
                this.rows.delete(id);
            }
            // decode a row entering or changing
            else {
                const columns = notification.table[TABLE];
                this.rows.set(id, columns.selectSchema().parse(columns.decode(change.row)));
            }
        }

        // keep each space and app's count, dropping groups that empty
        for (const result of page.results ?? []) {
            // drop every group
            if (result.group === null) {
                this.counts.clear();
            }
            // drop a group that empties
            else if (result.values === null) {
                const group = BadgeGroup.parse(result.group);
                this.counts.delete(`${group.space} ${group.packageId}`);
            }
            // keep a group's counts
            else {
                const group = BadgeGroup.parse(result.group);
                const values = BadgeValues.parse(result.values);
                this.counts.set(`${group.space} ${group.packageId}`, { ...group, ...values });
            }
        }
    }
}
