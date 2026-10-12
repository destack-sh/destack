import { Caller } from "@destack/access";
import { account } from "@destack/account/object";
import { Change, Snapshot, TABLE, type Table } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import type { Controller } from "@destack/service/control";
import { RequestId } from "@destack/service/request";
import {
    entitlement,
    product,
    productFeature,
    subscription,
    subscriptionItem,
} from "../object/index.ts";

/** Derives each account's entitlements and allowances as subscriptions or defaults change. */
export class EntitlementController implements Controller {
    /** The controller's name in reports. */
    readonly name = "entitlement";
    /** The account copies, subscriptions and default products entitlements derive from. */
    readonly watches: readonly Table[] = [
        account.table,
        subscription.table,
        subscriptionItem.table,
        product.table,
        productFeature.table,
    ];
    /** The object store deriving as its principal. */
    readonly #store: ObjectStore;
    /** The space keeping the entitlements, which the derivations run in. */
    readonly #home: string;

    /** Derive through a store keeping the entitlements in a space. */
    constructor(store: ObjectStore, home: string) {
        this.#store = store;
        this.#home = home;
    }

    /** Select the account of a changed copy, subscription or item, or all on a default's change. */
    async keys(change: Change): Promise<readonly string[]> {
        // key an account's copy by itself
        if (Change.of(change, account.table)) {
            return [Change.image(change).id];
        }
        // key a subscription by its account
        else if (Change.of(change, subscription.table)) {
            return [Change.image(change).accountId];
        }
        // key an item by its subscription's account, none once the subscription is deleted
        else if (Change.of(change, subscriptionItem.table)) {
            const parent = await Snapshot.live(this.#store.database).row(subscription.table, {
                id: Change.image(change).parentId,
            });

            return parent === null ? [] : [parent.accountId];
        }
        // key every account by a default product changing, which accounts without its seller take
        else if (Change.of(change, product.table)) {
            const isDefault = [Change.before(change), Change.after(change)].some(
                (image) => image?.isDefault === true,
            );

            return isDefault ? this.list() : [];
        }

        // key every account by a default product's grant changing, as subscriptions keep their own
        else if (Change.of(change, productFeature.table)) {
            const parent = await Snapshot.live(this.#store.database).row(product.table, {
                id: Change.image(change).parentId,
            });

            return parent !== null && parent.isDefault ? this.list() : [];
        }

        throw new TypeError(`the entitlement controller watches no ${change.table[TABLE].name}`);
    }

    /** List every account copied here. */
    async list(): Promise<readonly string[]> {
        const rows = await this.#store.database
            .select({ id: account.table.id })
            .from(account.table);

        return rows.map((row) => row.id);
    }

    /** Derive an account's entitlements and allowances in one mutation as the store's principal. */
    async reconcile(scope: string): Promise<number | undefined> {
        const now = this.#store.clock.now();
        await this.#store.mutate(
            Caller.of(this.#store.principal, now),
            {
                id: RequestId.create(now),
                calls: [
                    entitlement.call("derive", {
                        ...entitlement.in(this.#home),
                        accountId: account.identifier(scope),
                    }),
                ],
            },
            { scope: this.#home },
        );

        return undefined;
    }
}
