import { account } from "@destack/account/object";
import { type EventStore, EventTime } from "@destack/event";
import type { Call, IdentifierOf } from "@destack/object";
import type { PackageId } from "@destack/package";
import type { Identifier } from "@destack/schema";
import { isServiceError, ServiceError } from "@destack/service/error";
import { Catalog } from "../catalog/catalog.ts";
import { CatalogReference } from "../catalog/reference.ts";
import { Charge, Entitlement, type Publisher } from "../object/index.ts";
import type { Meter } from "./meter.ts";
import { type Period, type Usage, usage } from "./usage.ts";

/** How many usage events one rating pass takes: enough to keep a busy account's pass short. */
export const PAGE_EVENTS = 500;

/** The usage routed to the residency's paying accounts. */
export class Metering {
    /** The store keeping the usage events routed to their paying accounts. */
    readonly events: EventStore;
    /** Read the current catalog of a package declaring meters, SKUs and features. */
    readonly catalog: (packageId: PackageId) => Promise<Catalog>;
    /** Who publishes each package's SKUs to the home's buyers. */
    readonly publisher: Publisher;
    /** Report a use its declarations refuse. */
    readonly #report: (error: unknown) => void;

    /** Meter usage against the packages' current catalogs and publishers, reporting refusals. */
    constructor(
        events: EventStore,
        catalog: (packageId: PackageId) => Promise<Catalog>,
        publisher: Publisher,
        report: (error: unknown) => void,
    ) {
        // keep the usage store, the catalogs and the publishers
        this.events = events;
        this.catalog = catalog;
        this.publisher = publisher;
        this.#report = report;
    }

    /** Fold an account's usage of a meter, in a period or as kept now. */
    async read(
        scope: IdentifierOf<typeof account>,
        meter: Meter,
        period: Period | undefined,
    ): Promise<number> {
        // select the meter's usage of the account in the period
        const filter = {
            scope,
            where: {
                meter: CatalogReference.key({ packageId: meter.package.id, name: meter.name }),
            },
            ...(period === undefined
                ? {}
                : { from: EventTime.of(period.start), before: EventTime.of(period.end) }),
        };

        // fold the meter's usage as its aggregation reads it
        const series = await this.events.series(
            usage,
            filter,
            Metering.#request(meter.definition.aggregation, period),
        );

        return series
            .flatMap((entry) => entry.steps)
            .reduce((total, step) => total + step.value, 0);
    }

    /** Rate an account's usage past a log position, reporting uses that can never rate. */
    async rate(
        call: Call,
        accountId: Identifier<"account">,
        after: number,
    ): Promise<{ readonly sequence: number; readonly events: number }> {
        // read the account's usage committed past the position
        const page = await this.events.changes(usage, { scope: accountId }, after, PAGE_EVENTS);

        // parse the page's uses, reporting the ones their declarations refuse
        const catalog = Catalog.once(this.catalog);
        const admitted: Usage[] = [];
        for (const event of page.events) {
            const used = {
                ...event,
                keys: usage.parseKeys(event.keys),
                data: usage.parseData(event.data),
            };
            if (await this.#admits(used, catalog)) {
                admitted.push(used);
            }
        }

        // rate the uses of SKUs into their lines, reporting the ones their terms refuse
        const sold = admitted.filter((used) => used.keys.sku !== null);
        for (const refused of await Charge.rate(call, accountId, sold, catalog, this.publisher)) {
            this.#report(refused);
        }
        const metered = new Set(
            admitted.filter((used) => used.keys.sku === null).map((used) => used.keys.meter),
        );

        // rate the period's usage of each seller meter used into its metered prices
        for (const key of metered) {
            const reference = CatalogReference.of(key);
            const meter = (await catalog(reference.packageId)).meter(reference);
            await Charge.meter(call, accountId, meter, (period) =>
                this.read(accountId, meter, period),
            );
        }

        // derive the entitlements again from the counted usage
        await Entitlement.derive(call, accountId, this);

        return { sequence: page.sequence, events: page.events.length };
    }

    /** Request the series an aggregation reads, within a period or as kept now. */
    static #request(aggregation: Meter["definition"]["aggregation"], period: Period | undefined) {
        // keep the latest quantity of each source
        if (aggregation === "last") {
            return { measure: "quantity", fold: "last", group: ["source"] } as const;
        }
        // keep the level each source keeps now, without a period to average over
        else if (aggregation === "average" && period === undefined) {
            return { measure: "level", fold: "last", group: ["source"] } as const;
        }
        // add up the quantities of an averaged meter, to divide by the period
        else if (aggregation === "average") {
            return { measure: "quantity", fold: "sum" } as const;
        }

        // fold the quantities as the aggregation does
        return { measure: "quantity", fold: aggregation } as const;
    }

    /** Admit a use its declarations accept, reporting one they refuse. */
    async #admits(
        used: Usage,
        catalog: (packageId: PackageId) => Promise<Catalog>,
    ): Promise<boolean> {
        try {
            await requireUsage(used, catalog);

            return true;
        } catch (error) {
            // retry the page for a use that may pass later, as an unavailable catalog
            if (!isServiceError(error) || error.code === "SERVICE_UNAVAILABLE") {
                throw error;
            }

            // report a use that never passes
            this.#report(error);

            return false;
        }
    }
}

/** Refuse a use its meter or SKU declaration rejects. */
async function requireUsage(
    used: Usage,
    catalog: (packageId: PackageId) => Promise<Catalog>,
): Promise<void> {
    // read the use's meter from its package's current release
    const { packageId, name } = CatalogReference.of(used.keys.meter);
    const { level, sku: skuKey } = used.keys;
    const meter = (await catalog(packageId)).meter({ packageId, name });

    // require a level exactly for an averaged meter
    const isAveraged = meter.definition.aggregation === "average";
    if (isAveraged !== (level !== undefined && level !== null)) {
        const verb = isAveraged ? "needs a" : "takes no";
        throw new ServiceError("BAD_REQUEST", {
            message: `an event of meter ${name} ${verb} level, as its aggregation is ${meter.definition.aggregation}`,
        });
    }

    // require the SKU to price the use's meter, whose usage adds up over a period
    if (skuKey === null) {
        return;
    }
    const reference = CatalogReference.of(skuKey);
    const sku = (await catalog(reference.packageId)).sku(reference);
    const measured = CatalogReference.key(sku.definition.meter);
    if (measured !== CatalogReference.key({ packageId, name })) {
        throw new ServiceError("BAD_REQUEST", {
            message: `sku ${sku.name} prices meter ${measured}, not ${name}`,
        });
    } else if (!isAveraged && meter.definition.aggregation !== "sum") {
        throw new ServiceError("BAD_REQUEST", {
            message: `sku ${sku.name} rates no ${meter.definition.aggregation} meter`,
        });
    }
}
