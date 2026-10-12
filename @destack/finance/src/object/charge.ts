import { account, space } from "@destack/account/object";
import type { Select, Snapshot } from "@destack/db";
import { EventTime } from "@destack/event";
import { type Call, defineObject, field } from "@destack/object";
import type { PackageId } from "@destack/package";
import { found, type Identifier, present, schema, zip } from "@destack/schema";
import { isServiceError, ServiceError } from "@destack/service/error";
import type { Catalog } from "../catalog/catalog.ts";
import { CatalogReference } from "../catalog/reference.ts";
import type { Meter } from "../meter/meter.ts";
import { Period, type Usage } from "../meter/usage.ts";
import type { PaymentProvider } from "../provider/provider.ts";
import { CREDITS, Cost, Currency, DecimalAmount } from "../rate/amount.ts";
import { Rate } from "../rate/rate.ts";
import { SERVICE_CATEGORIES, type Sku } from "../sku/sku.ts";
import { BILLING_PERMISSIONS } from "./customer.ts";
import { price } from "./price.ts";
import { Product, product } from "./product.ts";
import { type Publisher, seller, Seller } from "./seller.ts";
import { type PeriodClose, Subscription, SubscriptionItem } from "./subscription.ts";

/** What a charge is for, as FOCUS ChargeCategory has it: usage, or prepaid usage as credit. */
export const CHARGE_CATEGORIES = ["Usage", "Credit"] as const;

/** One priced line an account owes a seller for a billing period, shaped as a FOCUS row. */
export const charge = defineObject({
    name: "charge",
    plural: "charges",
    scope: space,
    owner: "account",
    fields: {
        /** The account the row belongs to. */
        account: field.reference(account).relate("account"),
        /** The seller billing the line. */
        seller: field.reference(seller, { qualified: true }),
        /** What the line is for, the FOCUS ChargeCategory. */
        chargeCategory: field.enum(CHARGE_CATEGORIES),
        /** When the billing period the line belongs to started, the FOCUS ChargePeriodStart. */
        chargePeriodStart: field.time(),
        /** When that period ends, the FOCUS ChargePeriodEnd. */
        chargePeriodEnd: field.time(),

        /** The FOCUS ServiceCategory of the SKU used, absent beside usage. */
        serviceCategory: field.enum(SERVICE_CATEGORIES).optional(),
        /** The provider's service, the FOCUS ServiceName. */
        serviceName: field.string().optional(),
        /** The SKU used, by its package and name, the FOCUS SkuId. */
        skuId: field.string().optional(),
        /** The SKU's list price used, by the day it was read, the FOCUS SkuPriceId. */
        skuPriceId: field.string().optional(),
        /** The seller's metered price billing the usage, absent for a SKU's. */
        price: field.reference(price, { qualified: true }).optional(),
        /** The meter measuring the usage, the FOCUS SkuMeter. */
        skuMeter: field.string().optional(),
        /** The FOCUS RegionId the usage ran in, absent for a price every region shares. */
        regionId: field.string().optional(),
        /** The resource that used it, such as a space, the FOCUS ResourceId. */
        resourceId: field.string().optional(),

        /** The usage in the meter's unit, the FOCUS ConsumedQuantity. */
        consumedQuantity: field.number().optional(),
        /** The meter's unit, the FOCUS ConsumedUnit. */
        consumedUnit: field.string().optional(),
        /** The usage in the SKU's pricing unit, the FOCUS PricingQuantity. */
        pricingQuantity: field.number().optional(),
        /** The SKU's pricing unit, the FOCUS PricingUnit. */
        pricingUnit: field.string().optional(),
        /** The SKU's list price per pricing unit, the FOCUS ListUnitPrice. */
        listUnitPrice: field.string(DecimalAmount).optional(),

        /** The currency of the costs, the FOCUS BillingCurrency. */
        billingCurrency: field.string(Currency),
        /** The cost at the SKU's list price, the FOCUS ListCost. */
        listCost: field.string(Cost),
        /** The FOCUS BilledCost: the list cost within a subscription the provider bills. */
        billedCost: field.string(Cost),
        /** The billed cost once credit lines apply, the FOCUS EffectiveCost. */
        effectiveCost: field.string(Cost),

        /** Whether the line is open or invoiced. */
        status: field.enum(["open", "invoiced"]).default("open"),
        /** The payment provider's identifier of the invoice item, once invoiced. */
        providerId: field.string().optional(),
    },
    permissions: BILLING_PERMISSIONS,
    reserved: ["provide"],
    constraints: {
        usage: {
            kind: "index",
            on: ["accountId", "chargePeriodStart", "resourceId", "skuId"],
            unique: true,
            where: { chargeCategory: "Usage" },
        },
        price: {
            kind: "index",
            on: ["accountId", "chargePeriodStart", "price"],
            unique: true,
            where: { chargeCategory: "Usage", skuId: { isNull: true } },
        },
        credit: {
            kind: "index",
            on: ["accountId", "seller", "chargePeriodStart"],
            unique: true,
            where: { chargeCategory: "Credit" },
        },
        sku: { kind: "index", on: ["skuId", "status"] },
        period: {
            kind: "check",
            condition: { chargePeriodStart: { lt: { field: "chargePeriodEnd" } } },
        },
    },
}).methods((method) => ({
    get: method.get("read"),
    list: method.list("read"),
    create: method.create("provide", { isInternal: true }),
    update: method.update("provide", {
        isInternal: true,
        fields: [
            "skuPriceId",
            "consumedQuantity",
            "pricingQuantity",
            "listUnitPrice",
            "listCost",
            "billedCost",
            "effectiveCost",
            "status",
            "providerId",
        ],
    }),
    rate: method.mutation({
        permission: "provide",
        target: "none",
        isInternal: true,
        input: schema.object({
            /** The account whose usage is rated. */
            accountId: schema.identifier("account"),
            /** The log position the account's usage is rated up to. */
            after: schema.int().nonnegative(),
        }),
        output: schema.object({
            /** The log position the rating reached. */
            sequence: schema.int().nonnegative(),
            /** The usage events rated. */
            events: schema.int().nonnegative(),
        }),
    }),
}));
/** A persisted charge. */
export type Charge = Select<typeof charge.table>;

/** What a seller rates an account's usage at. */
interface Terms {
    /** The billing period the usage falls in. */
    readonly period: Period;
    /** The currency the usage is billed in. */
    readonly currency: string;
    /** The subscription billing the usage with its included usage, absent if the seller pays. */
    readonly subscription:
        | { readonly row: Subscription; readonly includedUsage: number }
        | undefined;
}

/** The priced lines accounts owe sellers: rated on arrival, closed and invoiced per period. */
export const Charge = {
    /** Rate a page of uses of SKUs into their resources' lines, returning the refused uses. */
    async rate(
        call: Call,
        accountId: Identifier<"account">,
        uses: readonly Usage[],
        catalog: (packageId: PackageId) => Promise<Catalog>,
        publisher: Publisher,
    ): Promise<ServiceError<string, unknown>[]> {
        // read the account's terms once for the page
        const snapshot = call.view();
        const granting = await Subscription.granting(snapshot, accountId);
        const publishers = new Map<string, Promise<Seller>>();
        const tallies = new Map<string, Tally>();
        const refused: ServiceError<string, unknown>[] = [];

        // add each use to its line's tally, refusing one its terms reject
        for (const used of uses) {
            try {
                await tally(snapshot, accountId, used, {
                    now: call.now,
                    catalog,
                    publisher,
                    granting,
                    publishers,
                    tallies,
                });
            } catch (error) {
                refused.push(refusal(error));
            }
        }

        // price each line's usage at the SKU's list price, capped by the spending limit
        const spent = new Map<string, Map<string, bigint>>();
        const calls = [];
        for (const [key, counted] of tallies) {
            const { sku, sold, terms, line } = counted;
            const conversion = convert(sold, sku.definition.currency, terms.currency);
            const isSubscribed = terms.subscription !== undefined;
            const rated = Rate.usage(sku, counted.consumed, isSubscribed, conversion);
            const billedCost = await capped(
                snapshot,
                terms.subscription,
                line?.id ?? key,
                rated.billedCost,
                spent,
            );
            const costs = {
                skuPriceId: rated.skuPriceId,
                consumedQuantity: counted.consumed,
                pricingQuantity: rated.pricingQuantity,
                listUnitPrice: rated.listUnitPrice,
                listCost: rated.listCost,
                billedCost,
                effectiveCost: billedCost,
            };

            // update the line, or open it with the SKU's FOCUS columns
            const { definition } = sku;
            calls.push(
                line === undefined
                    ? charge.call("create", {
                          accountId,
                          seller: { scope: sold.scope, id: sold.id },
                          chargeCategory: "Usage",
                          chargePeriodStart: terms.period.start,
                          chargePeriodEnd: terms.period.end,
                          serviceCategory: definition.category,
                          serviceName: definition.service,
                          skuId: counted.skuId,
                          skuMeter: counted.meter.name,
                          regionId: definition.region ?? null,
                          resourceId: counted.resourceId,
                          consumedUnit: counted.meter.definition.unit,
                          pricingUnit: definition.pricingUnit,
                          billingCurrency: terms.currency,
                          ...costs,
                      })
                    : charge.call("update", { id: line.id, ...costs }),
            );
        }
        await call.mutate(call.service, calls);

        return refused;
    },

    /** Rate the period's usage of a seller's meter into the line of each metered price. */
    async meter(
        call: Call,
        accountId: Identifier<"account">,
        meter: Meter,
        used: (period: { readonly start: number; readonly end: number }) => Promise<number>,
    ): Promise<void> {
        // read the granting subscriptions' items billing the meter
        const snapshot = call.view();
        const key = CatalogReference.key(meter.reference);
        const billing = (await Subscription.granting(snapshot, accountId)).flatMap((row) =>
            row.items
                .filter(
                    ({ terms: { recurring } }) =>
                        recurring.usage === "metered" &&
                        CatalogReference.key(recurring.meter) === key,
                )
                .map((item) => ({ row, item })),
        );
        const spent = new Map<string, Map<string, bigint>>();
        for (const { row, item } of billing) {
            requireRunning(row, call.now);

            // price the period's usage at the item's terms, at most what the spending limit leaves
            const period = { start: row.currentPeriodStart, end: row.currentPeriodEnd };
            const quantity = await used(period);
            const line = await readPriceLine(snapshot, row, item.price.id);
            const listCost = Rate.price(item.terms, quantity);
            const included = SubscriptionItem.includedUsage(row.items);
            const terms = { row, includedUsage: included };
            const billedCost = await capped(snapshot, terms, line?.id ?? item.id, listCost, spent);
            const costs = {
                consumedQuantity: quantity,
                pricingQuantity: quantity,
                listUnitPrice: Rate.unitPrice(item.terms),
                listCost,
                billedCost,
                effectiveCost: billedCost,
            };

            // update the line, or open it named after the price's product
            if (line !== undefined) {
                await call.mutate(call.service, [charge.call("update", { id: line.id, ...costs })]);
            } else {
                await call.mutate(call.service, [
                    charge.call("create", {
                        accountId,
                        seller: row.seller,
                        chargeCategory: "Usage",
                        chargePeriodStart: period.start,
                        chargePeriodEnd: period.end,
                        serviceName: await readProductName(snapshot, item.price.id),
                        skuMeter: meter.name,
                        price: item.price,
                        consumedUnit: meter.definition.unit,
                        pricingUnit: meter.definition.unit,
                        billingCurrency: item.terms.currency,
                        ...costs,
                    }),
                ]);
            }
        }
    },

    /** Read the usage cost of a subscription's current period so far, in minor units. */
    async spend(snapshot: Snapshot, row: Subscription): Promise<number> {
        const lines = await readLines(snapshot, row, row.currentPeriodStart, false);

        return Number(
            Cost.of(lines.reduce((total, line) => total + Cost.scaled(line.billedCost), 0n)),
        );
    },

    /** Add a period's open lines and their drawn included usage to a provider invoice. */
    async bill(
        snapshot: Snapshot,
        row: Subscription,
        period: Period,
        invoice: { readonly customer: string; readonly providerId: string | null },
        provider: PaymentProvider,
        key: string,
    ): Promise<PeriodClose> {
        // draw the period's included usage on its open usage lines
        const { lines, amounts, total, currency, includedUsage } = await draw(
            snapshot,
            row,
            period.start,
        );
        const drawn = Math.min(includedUsage, total);

        // add each line billing an amount, then the credit for what was drawn
        const billed = {
            customer: invoice.customer,
            invoice: invoice.providerId,
            currency,
            period,
        };
        const added: Record<string, string | null> = {};
        for (const [line, amount] of zip(lines, amounts)) {
            const item = { ...billed, amount, description: describe(line), charge: line.id };
            added[line.id] = amount > 0 ? await provider.bill(item, `${key}/${line.id}`) : null;
        }
        const credit = {
            ...billed,
            amount: -drawn,
            description: "Included usage",
            charge: null,
        };
        const creditLine = drawn === 0 ? null : await provider.bill(credit, `${key}/credit`);

        return { lines: added, drawn, creditLine };
    },

    /** Mark a period's lines invoiced exactly as billed, and keep their drawn included usage. */
    async close(call: Call, row: Subscription, period: Period, closed: PeriodClose): Promise<void> {
        // refuse a period whose open lines differ from those billed
        const { lines, currency } = await draw(call.view(), row, period.start);
        const billed = new Map(Object.entries(closed.lines));
        if (lines.length !== billed.size || lines.some((line) => !billed.has(line.id))) {
            throw new ServiceError("CONFLICT", {
                message: `the usage lines of subscription ${row.id} changed since its period was billed`,
            });
        }

        // mark each billed line invoiced with its invoice line
        await call.mutate(
            call.service,
            lines.map((line) =>
                charge.call("update", {
                    id: line.id,
                    status: "invoiced",
                    providerId: found(billed, line.id),
                }),
            ),
        );

        // keep the included usage drawn as a negative credit line
        if (closed.drawn > 0) {
            const drawn = Cost.of(-Cost.whole(closed.drawn));
            await call.mutate(call.service, [
                charge.call("create", {
                    accountId: row.accountId,
                    seller: row.seller,
                    chargeCategory: "Credit",
                    chargePeriodStart: period.start,
                    chargePeriodEnd: period.end,
                    billingCurrency: currency,
                    listCost: "0",
                    billedCost: drawn,
                    effectiveCost: drawn,
                    status: "invoiced",
                    providerId: closed.creditLine,
                }),
            ]);
        }
    },
};

/** The usage a page adds to one line, with what prices it. */
interface Tally {
    /** The SKU pricing the usage. */
    readonly sku: Sku;
    /** The meter measuring it. */
    readonly meter: Meter;
    /** The seller reselling the SKU. */
    readonly sold: Seller;
    /** What the seller rates the account's usage at. */
    readonly terms: Terms;
    /** The SKU's key. */
    readonly skuId: string;
    /** The resource using it. */
    readonly resourceId: string;
    /** The open line kept, absent before the period's first use. */
    readonly line: Charge | undefined;
    /** The line's quantity with the page's uses. */
    consumed: number;
}

/** Add a use of a SKU to the tally of its resource's line in the period. */
async function tally(
    snapshot: Snapshot,
    accountId: Identifier<"account">,
    used: Usage,
    page: {
        readonly now: number;
        readonly catalog: (packageId: PackageId) => Promise<Catalog>;
        readonly publisher: Publisher;
        readonly granting: Awaited<ReturnType<typeof Subscription.granting>>;
        readonly publishers: Map<string, Promise<Seller>>;
        readonly tallies: Map<string, Tally>;
    },
): Promise<void> {
    // read the SKU and its meter, and the reselling seller once per package
    const { sku: skuKey, meter: meterKey, quantity } = used.keys;
    const reference = CatalogReference.of(present(skuKey, "a use's SKU"));
    const metered = CatalogReference.of(meterKey);
    const sku = (await page.catalog(reference.packageId)).sku(reference);
    const meter = (await page.catalog(metered.packageId)).meter(metered);
    const publisher =
        page.publishers.get(sku.package.name) ??
        Seller.publisher(snapshot, sku.package, page.publisher);
    page.publishers.set(sku.package.name, publisher);
    const sold = await publisher;

    // decide the terms at the use's time, refusing usage into an ended period
    const terms = termsOf(page.granting, sold, EventTime.milliseconds(used.time), sku);
    if (terms.subscription !== undefined) {
        requireRunning(terms.subscription.row, page.now);
    }

    // add the quantity to the line's tally, reading the line once
    const skuId = CatalogReference.key(reference);
    const key = [terms.period.start, used.source, skuId].join(" ");
    const kept = page.tallies.get(key);
    if (kept !== undefined) {
        kept.consumed += quantity;
    } else {
        const line = await readLine(snapshot, accountId, terms.period.start, used.source, skuId);
        const consumed =
            line === undefined ? 0 : present(line.consumedQuantity, "a line's quantity");
        page.tallies.set(key, {
            sku,
            meter,
            sold,
            terms,
            skuId,
            resourceId: used.source,
            line,
            consumed: consumed + quantity,
        });
    }
}

/** Keep a refused use's error, retrying the page for a use that may rate later. */
function refusal(error: unknown): ServiceError<string, unknown> {
    if (!isServiceError(error) || error.code === "SERVICE_UNAVAILABLE") {
        throw error;
    }

    return error;
}

/** Convert a SKU's currency into a billing currency. */
function convert(sold: Seller, from: string, to: string): DecimalAmount {
    // keep a SKU listed in the billing currency at its figure
    if (from === to) {
        return "1";
    }

    // price credits at the seller's price of a credit in the billing currency
    const credit = from === CREDITS ? sold.creditPrices[to] : undefined;
    if (credit === undefined) {
        throw new ServiceError("PRECONDITION_FAILED", {
            message: `seller ${sold.id} prices no ${from} in ${to}`,
        });
    }

    return credit;
}

/** Read a resource's open usage line of a SKU. */
async function readLine(
    snapshot: Snapshot,
    accountId: Identifier<"account">,
    periodStart: number,
    resourceId: string,
    skuId: string,
): Promise<Charge | undefined> {
    const [line] = await snapshot.rows(charge.table, {
        accountId,
        chargePeriodStart: periodStart,
        resourceId,
        skuId,
        chargeCategory: "Usage",
    });

    return requireOpen(line);
}

/** Decide what a seller rates an account's usage at a time. */
function termsOf(
    granting: Awaited<ReturnType<typeof Subscription.granting>>,
    sold: Seller,
    time: number,
    sku: Sku,
): Terms {
    // find the account's granting subscription to the seller
    const chosen = granting.find((row) => row.seller.id === sold.id);

    // rate in the calendar month at the SKU's currency without a subscription
    if (chosen === undefined) {
        return {
            period: Period.month(time),
            currency: sku.definition.currency,
            subscription: undefined,
        };
    }

    // refuse usage of a period the subscription closed, and wait for one it has not started
    if (time < chosen.currentPeriodStart) {
        throw new ServiceError("BAD_REQUEST", {
            message: `usage at ${time} falls before the current period of subscription ${chosen.id}, which billed it already`,
        });
    } else if (time >= chosen.currentPeriodEnd) {
        throw new ServiceError("SERVICE_UNAVAILABLE", {
            message: `usage at ${time} falls after the period of subscription ${chosen.id}, retry once the next one starts`,
        });
    }

    // rate in the subscription's period and currency
    const { currency, includedUsage } = SubscriptionItem.billing(chosen);

    return {
        period: { start: chosen.currentPeriodStart, end: chosen.currentPeriodEnd },
        currency,
        subscription: { row: chosen, includedUsage },
    };
}

/** Cap a line at what the spending limit leaves after the period's other lines. */
async function capped(
    snapshot: Snapshot,
    billing: Terms["subscription"],
    key: string,
    billed: Cost,
    spent: Map<string, Map<string, bigint>>,
): Promise<Cost> {
    // bill a line without a spending limit as rated
    if (billing === undefined || billing.row.spendingLimit === null) {
        return billed;
    }

    // read the period's billed lines once per subscription
    const { row, includedUsage } = billing;
    const limit = billing.row.spendingLimit;
    const known = spent.get(row.id);
    const lines =
        known ??
        new Map(
            (await readLines(snapshot, row, row.currentPeriodStart, false)).map((line) => [
                line.id,
                Cost.scaled(line.billedCost),
            ]),
        );
    spent.set(row.id, lines);

    // bill what the included usage and the limit leave after the other lines
    const others = [...lines]
        .filter(([other]) => other !== key)
        .reduce((total, [, cost]) => total + cost, 0n);
    const left = Cost.whole(includedUsage + limit) - others;
    const room = left > 0n ? left : 0n;
    const rated = Cost.scaled(billed);
    const allowed = rated < room ? rated : room;
    lines.set(key, allowed);

    return Cost.of(allowed);
}

/** Read a subscription period's usage lines. */
async function readLines(
    snapshot: Snapshot,
    row: Subscription,
    periodStart: number,
    isOpen: boolean,
): Promise<Charge[]> {
    const rows = await snapshot.rows(charge.table, {
        accountId: row.accountId,
        chargePeriodStart: periodStart,
        chargeCategory: "Usage",
        ...(isOpen ? { status: "open" } : {}),
    });

    return rows.filter((line) => line.seller.id === row.seller.id);
}

/** Read a period's open lines in whole minor units, with the currency and included usage. */
async function draw(snapshot: Snapshot, row: Subscription, periodStart: number) {
    // round the lines to whole minor units adding up to their rounded total
    const lines = await readLines(snapshot, row, periodStart, true);
    const amounts = Cost.apportion(lines.map((line) => Cost.scaled(line.billedCost)));
    const total = amounts.reduce((added, amount) => added + amount, 0);

    // read the currency and included usage from the subscription's items
    const [itemized] = await Subscription.itemized(snapshot, [row]);

    return { lines, amounts, total, ...SubscriptionItem.billing(present(itemized, row.id)) };
}

/** Describe a usage line as its invoice line reads: the service, the quantity and the resource. */
function describe(line: Charge): string {
    // name the service, the quantity in its unit and the resource when one used it
    const quantity = present(line.pricingQuantity, "a usage line's quantity");
    const unit = present(line.pricingUnit, "a usage line's unit");
    const resource = line.resourceId === null ? "" : ` for ${line.resourceId}`;
    const service = present(line.serviceName, "a usage line's service");

    return `${service} ${quantity.toPrecision(6)} ${unit}${resource}`;
}

/** Refuse usage into a subscription period that ended, so its close never races new lines. */
function requireRunning(row: Subscription, now: number): void {
    // refuse once the clock passed the period's end
    if (now >= row.currentPeriodEnd) {
        throw new ServiceError("SERVICE_UNAVAILABLE", {
            message: `the period of subscription ${row.id} ended, retry once the next one starts`,
        });
    }
}

/** Read a subscription period's open usage line of a seller's metered price. */
async function readPriceLine(
    snapshot: Snapshot,
    row: Subscription,
    priceId: string,
): Promise<Charge | undefined> {
    // find the price's line, refusing one closed while the period moves on
    const lines = await readLines(snapshot, row, row.currentPeriodStart, false);

    return requireOpen(lines.find((each) => each.price?.id === priceId));
}

/** Refuse a usage line its period's close already invoiced, as the next period takes over. */
function requireOpen(line: Charge | undefined): Charge | undefined {
    if (line !== undefined && line.status !== "open") {
        throw new ServiceError("SERVICE_UNAVAILABLE", {
            message: `the period of charge ${line.id} is closed, retry once the next one starts`,
        });
    }

    return line;
}

/** Read the name of the product a price belongs to. */
async function readProductName(snapshot: Snapshot, priceId: string): Promise<string> {
    const priced = await snapshot.row(price.table, { id: price.identifier(priceId) });
    const sold =
        priced === null ? null : await snapshot.row(product.table, { id: priced.parentId });

    return Product.render(present(sold, `the product of price ${priceId}`)).name;
}
