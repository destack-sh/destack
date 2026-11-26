import { account } from "@destack/account/object";
import { EventView, type ObjectStore } from "@destack/object/server";
import { RequestId } from "@destack/sync";
import { customer, Customer, type Resolution } from "../object/index.ts";
import { usage } from "@destack/stack/object";
import type { Metering } from "./metering.ts";

/** Each customer's quotas, derived again as its usage, subscriptions or defaults change. */
export class CustomerQuota {
    /** The object store deriving as finance's installation. */
    readonly #store: ObjectStore;
    /** The platform's home, storing the customers, their quotas and their announcements. */
    readonly #home: string;
    /** The metering whose catalogs the quotas derive from. */
    readonly #metering: Metering;

    /** Derive through finance's store in the platform's home, with the metering's catalogs. */
    constructor(store: ObjectStore, home: string, metering: Metering) {
        this.#store = store;
        this.#home = home;
        this.#metering = metering;
    }

    /** Resolve each feature an account's grants allow now, with its metered usage. */
    resolve(scope: string): Promise<Resolution[]> {
        // read the catalogs, replicas and usage series the account's grants read from
        const caller = this.#store.as(customer);
        const releases = this.#metering.reader(this.#store, caller, this.#home);
        const reader = this.#store.database.snapshot();
        const events = new EventView(usage.eventKind(), { store: this.#store.events });

        return Customer.resolve(
            account.identifier(scope),
            releases,
            reader,
            events,
            this.#store.clock.now(),
        );
    }

    /** Announce an account's changed usage states, then write its quotas in the home. */
    async derive(scope: string): Promise<void> {
        // derive the account's quota changes and alerts, comparing them to its stored quotas
        const accountId = account.identifier(scope);
        const caller = this.#store.as(customer);
        const reader = this.#store.database.snapshot();
        const resolutions = await this.resolve(scope);
        const { calls, alerts } = await Customer.derive(accountId, resolutions, reader, this.#home);

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

        // write the changed quotas in the home as finance's installation
        if (calls.length > 0) {
            await this.#store.mutate(
                caller,
                { id: RequestId.create(this.#store.environment), calls },
                { scope: this.#home },
            );
        }
    }
}
