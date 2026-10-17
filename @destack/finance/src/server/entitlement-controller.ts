import { account } from "@destack/account/object";
import { Change, Snapshot, TABLE, type Table } from "@destack/db";
import { EventView } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import type { Controller } from "@destack/runtime/control";
import { RequestId } from "@destack/runtime/request";
import {
    customer,
    Customer,
    product,
    productFeature,
    subscription,
    subscriptionItem,
    usage,
} from "../object/index.ts";
import type { Metering } from "./metering.ts";

/** Derives each account's entitlements as subscriptions or defaults change. */
export class EntitlementController implements Controller {
    /** The controller's name in reports. */
    readonly name = "entitlement";
    /** The account replicas, subscriptions and default products entitlements derive from. */
    readonly watches: readonly Table[] = [
        account.table,
        subscription.table,
        subscriptionItem.table,
        product.table,
        productFeature.table,
    ];
    /** The object store deriving as finance's installation. */
    readonly #store: ObjectStore;
    /** The space storing the entitlements, which the derivations run in. */
    readonly #home: string;
    /** The metering whose catalogs the entitlements derive from. */
    readonly #metering: Metering;

    /** Derive through a store of the entitlements in a space, with the metering's catalogs. */
    constructor(store: ObjectStore, home: string, metering: Metering) {
        this.#store = store;
        this.#home = home;
        this.#metering = metering;
    }

    /** Select a change's accounts, all on a rare default edit: about 1 ms each, 100 s per 100k. */
    async keys(change: Change): Promise<readonly string[]> {
        // key an account's replica by itself
        if (Change.of(change, account.table)) {
            return [Change.image(change).id];
        }
        // key a subscription by its account
        else if (Change.of(change, subscription.table)) {
            return [Change.image(change).accountId];
        }
        // key an item by its subscription's account, none once the subscription is deleted
        else if (Change.of(change, subscriptionItem.table)) {
            const parent = await Snapshot.live(this.#store.database)
                .read(subscription.table)
                .findFirst({
                    where: {
                        id: Change.image(change).parentId,
                    },
                });

            return parent === undefined ? [] : [parent.accountId];
        }
        // key every account by a default product changing
        else if (Change.of(change, product.table)) {
            const isDefault = [Change.before(change), Change.after(change)].some(
                (image) => image?.isDefault === true,
            );

            return isDefault ? this.list() : [];
        }
        // key every account by a default product's grant changing, as subscriptions have their own
        else if (Change.of(change, productFeature.table)) {
            const parent = await Snapshot.live(this.#store.database)
                .read(product.table)
                .findFirst({
                    where: {
                        id: Change.image(change).parentId,
                    },
                });

            return parent !== undefined && parent.isDefault ? this.list() : [];
        }
        // refuse an unwatched table
        else {
            throw new TypeError(
                `table ${change.table[TABLE].name} is outside the entitlement controller's watches`,
            );
        }
    }

    /** List every account replicated here. */
    async list(): Promise<readonly string[]> {
        const rows = await this.#store.database
            .select({ id: account.table.id })
            .from(account.table);

        return rows.map((row) => row.id);
    }

    /** Derive an account's entitlements in one mutation, skipping a current one. */
    async reconcile(scope: string): Promise<number | undefined> {
        // skip an account whose stored entitlements are current, read without a mutation
        const now = this.#store.clock.now();
        const accountId = account.identifier(scope);
        const caller = this.#store.serving(customer);
        const catalog = this.#metering.catalog(this.#store, caller, this.#home);
        const reader = Snapshot.live(this.#store.database);
        const events = new EventView(usage.eventKind(), this.#store.events);
        if (await Customer.isEntitlementCurrent(accountId, catalog, reader, events, now)) {
            return undefined;
        }

        // derive the account's entitlements in one mutation
        await this.#store.mutate(
            caller,
            {
                id: RequestId.create(now),
                calls: [customer.call("entitle", { ...customer.in(this.#home), accountId })],
            },
            { scope: this.#home },
        );

        return undefined;
    }
}
