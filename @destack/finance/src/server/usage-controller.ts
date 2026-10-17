import { account } from "@destack/account/object";
import { Change, type Table, Transaction } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import type { Controller } from "@destack/runtime/control";
import { RequestId } from "@destack/runtime/request";
import { charge, PAGE_EVENTS, usage } from "../object/index.ts";

/** Rates each paying account's usage in commit order, moving a log slot in each transaction. */
export class UsageController implements Controller {
    /** The controller's name in reports. */
    readonly name = "usage";
    /** The usage arriving. */
    readonly watches: readonly Table[] = [usage.table];
    /** The object store rating the usage as finance's installation. */
    readonly #store: ObjectStore;
    /** The space storing the charges, which the ratings run in. */
    readonly #home: string;

    /** Rate the usage a home stores through `charge.rate`, in the space of the charges. */
    constructor(store: ObjectStore, home: string) {
        this.#store = store;
        this.#home = home;
    }

    /** Select the paying account whose usage arrived. */
    keys(change: Change): readonly string[] {
        return change.operation === "insert" && Change.of(change, usage.table)
            ? [Change.image(change).scope]
            : [];
    }

    /** List the paying accounts with a slot, which their first usage placed. */
    async list(): Promise<readonly string[]> {
        return usage.reader("usage").keys(this.#store.database);
    }

    /** Rate the next page of a paying account's usage and move its slot in one transaction. */
    async reconcile(payer: string): Promise<number | undefined> {
        // rate the page past the slot and move the slot in the same commit
        const reader = usage.reader("usage").at(payer);
        const now = this.#store.clock.now();
        const rated = await Transaction.run(this.#store.database, async (transaction) => {
            // rate the uses committed past where the account stands rated, as of this attempt
            const after = await reader.after(transaction.database);
            const [page] = await this.#store.mutate(
                this.#store.serving(charge),
                {
                    id: RequestId.create(now),
                    calls: [
                        charge.call("rate", {
                            ...charge.in(this.#home),
                            accountId: account.identifier(payer),
                            after,
                        }),
                    ],
                },
                { transaction, scope: this.#home },
            );

            // move the slot past the page
            if (page.sequence > after) {
                transaction.record((database) => reader.move(after, page.sequence, now, database));
            }

            return page.uses;
        });

        return rated === PAGE_EVENTS ? 0 : undefined;
    }
}
