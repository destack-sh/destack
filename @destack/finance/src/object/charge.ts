import { account } from "@destack/account/object";
import {
    and,
    check,
    type DatabaseConnection,
    eq,
    index,
    inArray,
    sql,
    uniqueIndex,
    type Select,
} from "@destack/db";
import { type Call, defineObject, field, type Invoke } from "@destack/object";
import type { PackageId } from "@destack/package";
import { present } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { Catalog } from "../catalog/catalog.ts";
import type { PaymentProvider } from "../provider/provider.ts";
import { CREDITS, Cost, Currency, DecimalAmount } from "../rate/amount.ts";
import { Rate } from "../rate/rate.ts";
import { SERVICE_CATEGORIES, type Sku } from "../sku/sku.ts";
import { BILLING_PERMISSIONS } from "./customer.ts";
import type { Meter } from "../meter/meter.ts";
import { Period, type Usage } from "../meter/usage.ts";
import { price } from "./price.ts";
import { product } from "./product.ts";
import { seller, Seller } from "./seller.ts";
import {
    GRANTING_STATES,
    type PeriodClose,
    subscription,
    type Subscription,
} from "./subscription.ts";
import { subscriptionItem } from "./subscription-item.ts";
import { CatalogReference } from "../catalog/reference.ts";

/** What a charge is for, as FOCUS ChargeCategory names it: usage, or the prepaid usage drawn on it as a credit. */
export const CHARGE_CATEGORIES = ["Usage", "Credit"] as const;

/** One priced line of what an account owes a seller for a billing period, as FOCUS shapes cost and usage rows. */
export const charge = defineObject({
    name: "charge",
    plural: "charges",
    scope: account,
    fields: {
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
        /** The provider's region the usage ran in, the FOCUS RegionId, absent for a price every region shares. */
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
        /** The amount billed, the list cost within a subscription the provider bills, the FOCUS BilledCost. */
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
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, { isSystem: true }),
        update: method.update(null, {
            isSystem: true,
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
    }),
    constraints: (entry) => [
        uniqueIndex("charge_usage")
            .on(entry.scope, entry.chargePeriodStart, entry.resourceId, entry.skuId)
            .where(sql`${entry.chargeCategory} = 'Usage'`),
        uniqueIndex("charge_price")
            .on(entry.scope, entry.chargePeriodStart, entry.price)
            .where(sql`${entry.chargeCategory} = 'Usage' AND ${entry.skuId} IS NULL`),
        uniqueIndex("charge_credit")
            .on(entry.scope, entry.chargePeriodStart)
            .where(sql`${entry.chargeCategory} = 'Credit'`),
        index("charge_sku").on(entry.skuId, entry.status),
        check("charge_period", sql`${entry.chargePeriodStart} < ${entry.chargePeriodEnd}`),
    ],
});
/** A persisted charge. */
export type Charge = Select<typeof charge.table>;

/** Work for one account as the system in an open transaction, such as rating its usage. */
export type AccountCall = Pick<Call, "database" | "scope" | "now"> & {
    /** Call object types' system methods in the account. */
    readonly invoke: Invoke;
};

/** What a seller rates an account's usage at. */
interface Terms {
    /** The billing period the usage falls in. */
    readonly period: Period;
    /** The currency the usage is billed in. */
    readonly currency: string;
    /** The subscription billing the usage, absent where the seller pays it. */
    readonly subscription: Subscription | undefined;
}

/** The priced lines accounts owe sellers: usage rated as it arrives, closed with each period and invoiced. */
export const Charge = {
    /** Rate a use into its resource's usage line. */
    async rate(
        call: AccountCall,
        usage: Usage,
        catalog: (packageId: PackageId) => Promise<Catalog>,
    ): Promise<void> {
        // leave a use of a seller's own meter, which no SKU rates
        const { sku: skuKey, meter: meterKey, quantity: value } = usage.keys;
        if (skuKey === null) {
            return;
        }

        // read the SKU and its meter, the reselling seller and the account's terms with it
        const { database } = call;
        const reference = CatalogReference.of(skuKey);
        const metered = CatalogReference.of(meterKey);
        const source = usage.source;
        const sku = (await catalog(reference.packageId)).sku(reference);
        const meter = (await catalog(metered.packageId)).meter(metered);
        const sold = await Seller.publisher(database, sku.package);
        const time = usage.time / 1000;
        const terms = await readTerms(database, call.scope, sold, time, sku);
        if (terms.subscription !== undefined) {
            requireRunning(terms.subscription, call.now);
        }

        // read the resource's usage line of the period
        const skuId = CatalogReference.key(reference);
        const line = await readLine(database, call.scope, terms.period.start, source, skuId);

        // price the period's usage at the SKU's list price in the terms' currency
        const consumed = (line?.consumedQuantity ?? 0) + value;
        const conversion = convert(sold, sku.definition.currency, terms.currency);
        const rated = Rate.usage(sku, consumed, terms.subscription !== undefined, conversion);
        const billedCost = await capped(database, terms.subscription, line, rated.billedCost);
        const costs = {
            skuPriceId: rated.skuPriceId,
            consumedQuantity: consumed,
            pricingQuantity: rated.pricingQuantity,
            listUnitPrice: rated.listUnitPrice,
            listCost: rated.listCost,
            billedCost,
            effectiveCost: billedCost,
        };

        // update the line, or open it with the SKU's FOCUS columns
        const { definition } = sku;
        if (line !== undefined) {
            await call.invoke(charge).update({ id: line.id, ...costs });
        } else {
            await call.invoke(charge).create({
                seller: { scope: sold.scope, id: sold.id },
                chargeCategory: "Usage",
                chargePeriodStart: terms.period.start,
                chargePeriodEnd: terms.period.end,
                serviceCategory: definition.category,
                serviceName: definition.service,
                skuId,
                skuMeter: meter.name,
                regionId: definition.region ?? null,
                resourceId: source,
                consumedUnit: meter.definition.unit,
                pricingUnit: definition.pricingUnit,
                billingCurrency: terms.currency,
                ...costs,
            });
        }
    },

    /** Read what a subscription's period costs its account in usage so far, in fractional minor units. */
    /** Rate the period's usage of a seller's meter into the line of each metered price billing it. */
    async meter(
        call: AccountCall,
        meter: Meter,
        used: (period: { readonly start: number; readonly end: number }) => Promise<number>,
    ): Promise<void> {
        // read the granting subscriptions' items billing the meter
        const { database } = call;
        const held = await database
            .select()
            .from(subscription.table)
            .where(
                and(
                    eq(subscription.table.scope, account.identifier(call.scope)),
                    inArray(subscription.table.status, GRANTING_STATES),
                ),
            );
        const items =
            held.length === 0
                ? []
                : await database
                      .select()
                      .from(subscriptionItem.table)
                      .where(
                          inArray(
                              subscriptionItem.table.parentId,
                              held.map((row) => row.id),
                          ),
                      );
        const key = CatalogReference.key(meter.reference);
        for (const item of items) {
            const { recurring } = item.terms;
            const row = held.find((each) => each.id === item.parentId);
            if (
                row === undefined ||
                recurring?.usage !== "metered" ||
                CatalogReference.key(recurring.meter) !== key
            ) {
                continue;
            }
            requireRunning(row, call.now);

            // price the period's usage at the item's terms, at most what the spending limit leaves
            const period = { start: row.currentPeriodStart, end: row.currentPeriodEnd };
            const quantity = await used(period);
            const line = await readPriceLine(database, row, item.price.id);
            const listCost = Rate.price(item.terms, quantity);
            const billedCost = await capped(database, row, line, listCost);
            const costs = {
                consumedQuantity: quantity,
                pricingQuantity: quantity,
                listUnitPrice:
                    item.terms.unitAmountDecimal ??
                    (item.terms.unitAmount === null ? null : String(item.terms.unitAmount)),
                listCost,
                billedCost,
                effectiveCost: billedCost,
            };

            // update the line, or open it named after the price's product
            if (line !== undefined) {
                await call.invoke(charge).update({ id: line.id, ...costs });
            } else {
                await call.invoke(charge).create({
                    seller: row.seller,
                    chargeCategory: "Usage",
                    chargePeriodStart: period.start,
                    chargePeriodEnd: period.end,
                    serviceName: await readProductName(database, item.price.id),
                    skuMeter: meter.name,
                    price: item.price,
                    consumedUnit: meter.definition.unit,
                    pricingUnit: meter.definition.unit,
                    billingCurrency: item.terms.currency,
                    ...costs,
                });
            }
        }
    },

    async spend(database: DatabaseConnection, row: Subscription): Promise<number> {
        const lines = await readLines(database, row, row.currentPeriodStart, false);

        return Number(
            Cost.of(lines.reduce((total, line) => total + Cost.scaled(line.billedCost), 0n)),
        );
    },

    /** Add a period's open lines and the included usage drawn on them to an invoice of the provider. */
    async bill(
        database: DatabaseConnection,
        row: Subscription,
        period: Period,
        invoice: { readonly customer: string; readonly providerId: string | null },
        provider: PaymentProvider,
        key: string,
    ): Promise<PeriodClose> {
        // draw the period's included usage on its open usage lines
        const { lines, amounts, total, currency } = await draw(database, row, period.start);
        const drawn = Math.min(row.includedUsage, total);

        // add each line billing an amount, then the credit for what was drawn
        const billed = {
            customer: invoice.customer,
            invoice: invoice.providerId,
            currency,
            period,
        };
        const added: Record<string, string> = {};
        for (const [position, line] of lines.entries()) {
            const amount = present(amounts[position], "a line's amount");
            if (amount > 0) {
                const item = { ...billed, amount, description: describe(line), charge: line.id };
                added[line.id] = await provider.bill(item, `${key}/${line.id}`);
            }
        }
        const credit = {
            ...billed,
            amount: -drawn,
            description: "Included usage",
            charge: `${row.id}/credit`,
        };
        const creditLine = drawn === 0 ? null : await provider.bill(credit, `${key}/credit`);

        return { lines: added, drawn, creditLine };
    },

    /** Mark a period's lines invoiced as billed, and keep the included usage drawn on them. */
    async close(
        call: Pick<Call, "database" | "invoke">,
        row: Subscription,
        period: Period,
        closed: PeriodClose,
    ): Promise<void> {
        // mark each open usage line of the period invoiced
        const { lines, currency } = await draw(call.database, row, period.start);
        for (const line of lines) {
            const providerId = closed.lines[line.id] ?? null;
            await call.invoke(charge).update({ id: line.id, status: "invoiced", providerId });
        }

        // keep the included usage drawn as a negative credit line
        if (closed.drawn > 0) {
            const drawn = Cost.of(-Cost.whole(closed.drawn));
            await call.invoke(charge).create({
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
            });
        }
    },
};

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
    database: DatabaseConnection,
    scope: string,
    periodStart: number,
    resourceId: string,
    skuId: string,
): Promise<Charge | undefined> {
    const [line] = await database
        .select()
        .from(charge.table)
        .where(
            and(
                eq(charge.table.scope, account.identifier(scope)),
                eq(charge.table.chargePeriodStart, periodStart),
                eq(charge.table.resourceId, resourceId),
                eq(charge.table.skuId, skuId),
                eq(charge.table.chargeCategory, "Usage"),
            ),
        );
    if (line !== undefined && line.status !== "open") {
        throw new ServiceError("SERVICE_UNAVAILABLE", {
            message: `the period of charge ${line.id} is closed, retry once the next one starts`,
        });
    }

    return line;
}

/** Read what a seller rates an account's usage at. */
async function readTerms(
    database: DatabaseConnection,
    scope: string,
    sold: Seller,
    time: number,
    sku: Sku,
): Promise<Terms> {
    // read the account's granting subscriptions to the seller
    const rows = await database
        .select()
        .from(subscription.table)
        .where(
            and(
                eq(subscription.table.scope, account.identifier(scope)),
                inArray(subscription.table.status, GRANTING_STATES),
            ),
        );
    const chosen = rows.find((row) => row.seller.id === sold.id);

    // rate in the calendar month at the SKU's currency without a subscription
    if (chosen === undefined) {
        return {
            period: Period.month(time),
            currency: sku.definition.currency,
            subscription: undefined,
        };
    }

    // rate in the subscription's period and currency
    return {
        period: { start: chosen.currentPeriodStart, end: chosen.currentPeriodEnd },
        currency: await readCurrency(database, chosen.id),
        subscription: chosen,
    };
}

/** Cap a line at what the spending limit leaves, forgiving the rest. */
async function capped(
    database: DatabaseConnection,
    row: Subscription | undefined,
    line: Charge | undefined,
    billed: Cost,
): Promise<Cost> {
    // bill a line without a spending limit as rated
    if (row === undefined || row.spendingLimit === null) {
        return billed;
    }

    // bill what the included usage and the limit leave after the period's other lines
    const others = await readLines(database, row, row.currentPeriodStart, false);
    const spent = others
        .filter((other) => other.id !== line?.id)
        .reduce((total, other) => total + Cost.scaled(other.billedCost), 0n);
    const left = Cost.whole(row.includedUsage + row.spendingLimit) - spent;
    const rated = Cost.scaled(billed);

    return Cost.of(rated < left ? rated : left > 0n ? left : 0n);
}

/** Read the currency a subscription bills in, its prices'. */
async function readCurrency(database: DatabaseConnection, subscriptionId: string): Promise<string> {
    const [item] = await database
        .select({ terms: subscriptionItem.table.terms })
        .from(subscriptionItem.table)
        .where(eq(subscriptionItem.table.parentId, subscription.identifier(subscriptionId)))
        .limit(1);

    return present(item, `an item of subscription ${subscriptionId}`).terms.currency;
}

/** Read a subscription period's usage lines. */
async function readLines(
    database: DatabaseConnection,
    row: Subscription,
    periodStart: number,
    isOpen: boolean,
): Promise<Charge[]> {
    const rows = await database
        .select()
        .from(charge.table)
        .where(
            and(
                eq(charge.table.scope, row.scope),
                eq(charge.table.chargePeriodStart, periodStart),
                eq(charge.table.chargeCategory, "Usage"),
                isOpen ? eq(charge.table.status, "open") : undefined,
            ),
        );

    return rows.filter((line) => line.seller.id === row.seller.id);
}

/** Read a period's open lines, their amounts in whole minor units and their currency. */
async function draw(database: DatabaseConnection, row: Subscription, periodStart: number) {
    // add up the lines rounded up to whole minor units
    const lines = await readLines(database, row, periodStart, true);
    const amounts = lines.map((line) => Cost.ceiling(Cost.scaled(line.billedCost)));
    const total = amounts.reduce((added, amount) => added + amount, 0);

    return { lines, amounts, total, currency: await readCurrency(database, row.id) };
}

/** Describe a usage line as its invoice line reads: the service, the quantity and the resource. */
function describe(line: Charge): string {
    // name the service, the quantity in its unit and the resource when one used it
    const quantity = present(line.pricingQuantity, "a usage line's quantity");
    const unit = present(line.pricingUnit, "a usage line's unit");
    const resource = line.resourceId === null ? "" : ` for ${line.resourceId}`;

    return `${line.serviceName ?? line.skuId} ${quantity.toPrecision(6)} ${unit}${resource}`;
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
    database: DatabaseConnection,
    row: Subscription,
    priceId: string,
): Promise<Charge | undefined> {
    // find the price's line, refusing one closed while the period moves on
    const lines = await readLines(database, row, row.currentPeriodStart, false);
    const line = lines.find((each) => each.price?.id === priceId);
    if (line !== undefined && line.status !== "open") {
        throw new ServiceError("SERVICE_UNAVAILABLE", {
            message: `the period of charge ${line.id} is closed, retry once the next one starts`,
        });
    }

    return line;
}

/** Read the name of the product a price belongs to. */
async function readProductName(database: DatabaseConnection, priceId: string): Promise<string> {
    const [row] = await database
        .select({ name: product.table.name })
        .from(price.table)
        .innerJoin(product.table, eq(product.table.id, price.table.parentId))
        .where(eq(price.table.id, price.identifier(priceId)));

    return present(row, `the product of price ${priceId}`).name;
}
