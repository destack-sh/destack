import { account } from "@destack/account/object";
import { Change, Snapshot, TABLE, type Table } from "@destack/db";
import { EventView } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import type { ReconcileController } from "@destack/db";
import { RequestId } from "@destack/sync";
import {
    customer,
    Customer,
    type Resolution,
    product,
    productFeature,
    subscription,
    subscriptionItem,
    usage,
} from "../object/index.ts";
import type { Metering } from "./metering.ts";

/** Derives each account's entitlements as its usage, subscriptions or defaults change. */
export class EntitlementController implements ReconcileController {
    /** The controller's name in reports. */
    readonly name = "entitlement";
    /** The account replicas, subscriptions and default products entitlements derive from. */
    readonly watches: readonly Table[] = [
        account.table,
        usage.table,
        subscription.table,
        subscriptionItem.table,
        product.table,
        productFeature.table,
    ];
    /** The object store deriving as finance's installation. */
    readonly #store: ObjectStore;
    /** The residency's home, storing the customers whose announcements alert the accounts. */
    readonly #home: string;
    /** The metering whose catalogs the entitlements derive from. */
    readonly #metering: Metering;

    /** Derive through finance's store in the residency's home, with the metering's catalogs. */
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
        // key arriving usage by its paying account
        else if (Change.of(change, usage.table)) {
            return change.operation === "insert" ? [Change.image(change).scope] : [];
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
                `table ${change.table[TABLE].name} is not watched by the entitlement controller`,
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

    /** Resolve each feature an account's sources grant now, with its metered usage. */
    resolve(scope: string): Promise<Resolution[]> {
        // read the catalogs, replicas and usage series the account's sources grant from
        const caller = this.#store.as(customer);
        const releases = this.#metering.reader(this.#store, caller, this.#home);
        const reader = Snapshot.live(this.#store.database);
        const events = new EventView(usage.eventKind(), this.#store.events);

        return Customer.resolve(
            account.identifier(scope),
            releases,
            reader,
            events,
            this.#store.clock.now(),
        );
    }

    /** Announce an account's changed usage states, then write its entitlements at the universe. */
    async reconcile(scope: string): Promise<number | undefined> {
        // derive the account's entitlements and alerts, comparing them to its replicated ones
        const accountId = account.identifier(scope);
        const caller = this.#store.as(customer);
        const reader = Snapshot.live(this.#store.database);
        const resolutions = await this.resolve(scope);
        const { calls, alerts } = await Customer.derive(accountId, resolutions, reader);

        // announce the alerts before writing, so no written state skips its announcement
        if (alerts.length > 0) {
            const announcing = customer.call("announce", {
                ...customer.in(this.#home),
                accountId,
                alerts,
            });
            await this.#store.mutate(
                caller,
                { id: RequestId.create(this.#store.environment), calls: [announcing] },
                { scope: this.#home },
            );
        }

        // write the changed entitlements at the universe as finance's installation
        if (calls.length > 0) {
            await this.#store.mutate(caller, {
                id: RequestId.create(this.#store.environment),
                calls,
            });
        }

        return undefined;
    }
}
