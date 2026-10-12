import { Caller } from "@destack/access";
import { account } from "@destack/account/object";
import { Change, type Table, Transaction } from "@destack/db";
import { EventReader } from "@destack/event";
import type { ObjectStore } from "@destack/object/server";
import type { Controller } from "@destack/service/control";
import { RequestId } from "@destack/service/request";
import { PAGE_EVENTS } from "../meter/metering.ts";
import { usage } from "../meter/usage.ts";
import { charge } from "../object/index.ts";

/** How long the log keeps unrated usage: thirty days, past any outage of rating passes. */
const HOLD_MILLISECONDS = 30 * 24 * 60 * 60 * 1000;

/** Rates each account's usage in commit order, moving a log slot in each transaction. */
export class UsageController implements Controller {
    /** The controller's name in reports. */
    readonly name = "usage";
    /** The usage arriving. */
    readonly watches: readonly Table[] = [usage.table];
    /** The object store rating the usage as its principal. */
    readonly #store: ObjectStore;
    /** The space keeping the charges, which the ratings run in. */
    readonly #home: string;

    /** Rate the usage a store receives through `charge.rate`, in the space of the charges. */
    constructor(store: ObjectStore, home: string) {
        this.#store = store;
        this.#home = home;
    }

    /** Select the account whose usage arrived. */
    keys(change: Change): readonly string[] {
        return change.operation === "insert" && Change.of(change, usage.table)
            ? [Change.image(change).scope]
            : [];
    }

    /** List every account with usage, each rated past its slot. */
    async list(): Promise<readonly string[]> {
        const rows = await this.#store.database
            .selectDistinct({ scope: usage.table.scope })
            .from(usage.table);

        return rows.map((row) => row.scope);
    }

    /** Rate the next page of an account's usage and move its slot in one transaction. */
    async reconcile(scope: string): Promise<number | undefined> {
        // rate the page past the slot and move the slot in the same commit
        const reader = UsageReader.of(scope);
        const now = this.#store.clock.now();
        const rated = await Transaction.run(this.#store.database, async (transaction) => {
            // read where the account's usage stands rated, as of this attempt
            const after = await reader.after(transaction.database);
            const [result] = await this.#store.mutate(
                Caller.of(this.#store.principal, now),
                {
                    id: RequestId.create(now),
                    calls: [
                        charge.call("rate", {
                            ...charge.in(this.#home),
                            accountId: account.identifier(scope),
                            after,
                        }),
                    ],
                },
                { transaction, scope: this.#home },
            );
            if (result.sequence > after) {
                transaction.record((database) =>
                    reader.move(database, after, result.sequence, now),
                );
            }

            return result;
        });

        return rated.events === PAGE_EVENTS ? 0 : undefined;
    }
}

/** The reader of an account's usage in commit order, whose slot keeps its unrated usage. */
export const UsageReader = {
    /** The reader of an account's usage. */
    of(scope: string): EventReader {
        return new EventReader(`usage:${scope}`, HOLD_MILLISECONDS);
    },
};
