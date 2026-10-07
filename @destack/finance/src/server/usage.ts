import { account } from "@destack/account/object";
import { Change, type DatabaseConnection, type Table } from "@destack/db";
import { EventReader, type EventStore } from "@destack/event";
import type { IdentifierOf } from "@destack/object";
import type { ObjectServer } from "@destack/object/server";
import type { PackageId } from "@destack/package";
import { ServiceError } from "@destack/service/error";
import type { Controller } from "@destack/service/control";
import { Catalog } from "../catalog/catalog.ts";
import { CatalogReference } from "../catalog/reference.ts";
import type { Meter } from "../meter/meter.ts";
import { type Period, type Usage, usage } from "../meter/usage.ts";
import { type AccountCall, Charge } from "../object/index.ts";
import { asAccount } from "./account.ts";
import { measure } from "./entitlement.ts";

/** How many usage events one rating pass takes: enough to keep a busy account's pass short. */
const PAGE_EVENTS = 500;

/** How long the log keeps an account's unrated usage: thirty days, past any outage a rating pass may wait out. */
const HOLD_MILLISECONDS = 30 * 24 * 60 * 60 * 1000;

/** The usage an account's usage events add up to, read from the store keeping them. */
export class MeterUsage {
    /** The store keeping the usage events routed to their paying accounts. */
    readonly store: EventStore;

    /** Read usage from a store keeping usage events. */
    constructor(store: EventStore) {
        this.store = store;
    }

    /** Fold an account's usage of a meter, in a period or as kept now. */
    async read(
        scope: IdentifierOf<typeof account>,
        meter: Meter,
        period: Period | undefined,
        connection?: DatabaseConnection,
    ): Promise<number> {
        // select the meter's usage of the account in the period
        const filter = {
            scope,
            where: {
                meter: CatalogReference.key({ packageId: meter.package.id, name: meter.name }),
            },
            ...(period === undefined
                ? {}
                : { from: period.start * 1000, before: period.end * 1000 }),
        };

        // add up the latest value of each source, or the level each keeps now, else fold the quantities
        const aggregation = meter.definition.aggregation;
        const series =
            aggregation === "last" || (aggregation === "average" && period === undefined)
                ? await this.store.series(
                      usage,
                      filter,
                      {
                          measure: aggregation === "last" ? "quantity" : "level",
                          fold: "last",
                          group: ["source"],
                      },
                      connection,
                  )
                : await this.store.series(
                      usage,
                      filter,
                      {
                          measure: "quantity",
                          fold: aggregation === "average" ? "sum" : aggregation,
                      },
                      connection,
                  );

        return series
            .flatMap((entry) => entry.steps)
            .reduce((total, step) => total + step.value, 0);
    }
}

/** Rates each account's usage in commit order, past a log slot each rating advances in its own transaction. */
export class UsageController implements Controller {
    /** The controller's name in reports. */
    readonly name = "usage";
    /** The usage arriving. */
    readonly watches: readonly Table[] = [usage.table];
    /** The object server rating the usage as the system. */
    readonly #server: ObjectServer;
    /** The usage read back while rating. */
    readonly #meterUsage: MeterUsage;
    /** Read the current catalogs declaring meters and SKUs. */
    readonly #catalog: (packageId: PackageId) => Promise<Catalog>;
    /** Report a use its declarations refuse. */
    readonly #report: (error: unknown) => void;

    /** Rate the usage a store receives through a server. */
    constructor(
        server: ObjectServer,
        meterUsage: MeterUsage,
        catalog: (packageId: PackageId) => Promise<Catalog>,
        report: (error: unknown) => void,
    ) {
        // keep the server, the usage, the catalogs and the reporter
        this.#server = server;
        this.#meterUsage = meterUsage;
        this.#catalog = catalog;
        this.#report = report;
    }

    /** Select the account whose usage arrived. */
    keys(change: Change): readonly string[] {
        return change.operation === "insert" && Change.of(change, usage.table)
            ? [Change.image(change).scope]
            : [];
    }

    /** List every account holding usage, each rated past its slot. */
    async list(): Promise<readonly string[]> {
        const rows = await this.#server.database
            .selectDistinct({ scope: usage.table.scope })
            .from(usage.table);

        return rows.map((row) => row.scope);
    }

    /** Rate the next page of an account's usage, looking again at once while full pages remain. */
    async reconcile(scope: string): Promise<number | undefined> {
        // read the account's usage committed past its slot
        const after = await UsageReader.of(scope).after(this.#server.database);
        const page = await this.#meterUsage.store.changes(usage, { scope }, after, PAGE_EVENTS);
        if (page.sequence === after) {
            return undefined;
        }

        // rate the page and advance the slot past it in one transaction
        await asAccount(this.#server, scope, async (call) => {
            await this.#rate(call, page.events);
            await UsageReader.of(scope).advance(call.database, page.sequence, call.now);
        });

        return page.events.length === PAGE_EVENTS ? 0 : undefined;
    }

    /** Rate uses into charges and count them into the metered entitlements, reporting one its declarations refuse. */
    async #rate(call: AccountCall, events: readonly Usage[]): Promise<void> {
        // rate each valid use of a SKU, and collect the seller meters used
        const catalog = Catalog.once(this.#catalog);
        const metered = new Set<string>();
        for (const event of events) {
            const used = {
                ...event,
                keys: usage.parseKeys(event.keys),
                data: usage.parseData(event.data),
            };
            try {
                await requireUsage(used, catalog);
            } catch (error) {
                this.#report(error);
                continue;
            }
            await Charge.rate(call, used, catalog);
            if (used.keys.sku === null) {
                metered.add(used.keys.meter);
            }
        }

        // rate the period's usage of each seller meter used into its metered prices
        const scope = account.identifier(call.scope);
        for (const key of metered) {
            const reference = CatalogReference.of(key);
            const meter = (await catalog(reference.packageId)).meter(reference);
            await Charge.meter(call, meter, (period) =>
                this.#meterUsage.read(scope, meter, period, call.database),
            );
        }

        // count the usage into the metered entitlements
        await measure(call, this.#catalog, this.#meterUsage);
    }
}

/** The reader of an account's usage in commit order, whose slot keeps its unrated usage. */
export const UsageReader = {
    /** The reader of an account's usage. */
    of(scope: string): EventReader {
        return new EventReader(`usage:${scope}`, HOLD_MILLISECONDS);
    },
};

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
