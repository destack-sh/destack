import { type Select, TABLE } from "@destack/db";
import type { ObjectServer } from "@destack/object/server";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { subjectContext } from "@destack/service/test";
import type { Page, Subject } from "@destack/sync";
import { notification } from "../object/index.ts";
import { type Badge, UNREAD } from "../inbox/index.ts";

/** A row of a change, read for its identifier. */
const RowIdentity = schema.looseObject({ id: schema.string() });

/** A person's inbox in their home: its notifications and their live unread counts. */
export class HomeInbox {
    /** The rows kept, by identifier. */
    readonly rows = new Map<string, Select<typeof notification.table>>();
    /** The unread counts kept, by app. */
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
    constructor(server: Pick<ObjectServer, "source">, homeId: string, person: Subject) {
        // read every page in the background into the rows and counts kept
        const context = subjectContext(person, homeId, { signal: this.#following.signal });
        const { where, ...aggregate } = UNREAD;
        const pages = server.source.relayed(
            server.source.queriesShape.subscription({
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

    /** Read the unread count across apps, as a dock badge shows it. */
    get unread(): number {
        return [...this.counts.values()].reduce((total, badge) => total + badge.unread, 0);
    }

    /** End the stream and wait for its last page. */
    async close(): Promise<void> {
        this.#following.abort();
        await this.#reading;
    }

    /** Follow the notifications kept, the latest occurrence first. */
    async *notifications(
        signal: AbortSignal,
    ): AsyncIterable<readonly Select<typeof notification.table>[]> {
        while (!signal.aborted) {
            yield [...this.rows.values()].toSorted(
                (left, right) => right.occurredAt - left.occurredAt,
            );
            await this.#next(this.#pages + 1);
        }
    }

    /** Follow the unread counts kept. */
    async *badges(signal: AbortSignal): AsyncIterable<readonly Badge[]> {
        while (!signal.aborted) {
            yield [...this.counts.values()];
            await this.#next(this.#pages + 1);
        }
    }

    /** Wait until the kept rows and counts meet a condition and read each arriving page. */
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

        // keep each app's count, dropping groups that empty
        for (const result of page.results ?? []) {
            if (result.group === null) {
                this.counts.clear();
            } else if (result.values === null) {
                this.counts.delete(String(result.group["packageId"]));
            } else {
                this.counts.set(String(result.group["packageId"]), {
                    space: schema.string().parse(result.group["space"]),
                    packageId: PackageId.parse(result.group["packageId"]),
                    unread: Number(result.values["unread"]),
                    latestAt: Number(result.values["latestAt"]),
                });
            }
        }
    }
}
