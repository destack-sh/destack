import { account } from "@destack/account/object";
import { Snapshot } from "@destack/db";
import { EventView } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { RequestId } from "@destack/sync";
import { customer, Customer, type Resolution } from "../object/index.ts";
import { usage } from "@destack/stack/object";
import type { Metering } from "./metering.ts";

/** Each customer's entitlements, derived again as its usage, subscriptions or defaults change. */
export class CustomerEntitlement {
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
    async derive(scope: string): Promise<void> {
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
    }
}
