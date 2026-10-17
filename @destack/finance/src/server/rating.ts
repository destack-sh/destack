import { type Call, ObjectReference } from "@destack/sync";
import { EventTime } from "@destack/event";
import type { MethodContext } from "@destack/object";
import type { PackageId } from "@destack/package";
import { type Identifier, present } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import type { Catalog } from "../catalog/catalog.ts";
import { CatalogReference } from "../catalog/reference.ts";
import type { Meter } from "@destack/observability/meter";
import { Period } from "../rate/period.ts";
import {
    charge,
    Charge,
    price,
    Product,
    product,
    type Publisher,
    seller,
    Seller,
    Subscription,
    SubscriptionItem,
    Usage,
} from "../object/index.ts";
import { CREDITS, Cost, type DecimalAmount } from "../rate/amount.ts";
import { Rate } from "../rate/rate.ts";
import type { Sku } from "../sku/sku.ts";

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
    /** The open line, absent before the period's first use. */
    readonly line: Charge | undefined;
    /** The line's quantity with the page's uses. */
    consumed: number;
}

/** The rating of one page of a paying account's usage into its charge lines. */
export class Rating {
    /** The paying account. */
    readonly accountId: Identifier<"account">;
    /** The account's granting subscriptions with their items, read once for the page. */
    readonly granting: Awaited<ReturnType<typeof Subscription.granting>>;
    /** Read the current catalog of a package. */
    readonly #catalog: (packageId: PackageId) => Promise<Catalog>;
    /** Who publishes each package's SKUs to the home's buyers. */
    readonly #publisher: Publisher;
    /** The call the page is rated in. */
    readonly #context: MethodContext;
    /** The open lines of each subscription's current period, by subscription. */
    readonly #lines: Map<string, readonly Charge[]>;
    /** The billed cost of each line of a subscription's period as the page rates it. */
    readonly #spent: Map<string, Map<string, bigint>>;

    /** Rate a page of an account's usage under the terms it has now. */
    private constructor(
        accountId: Identifier<"account">,
        granting: Awaited<ReturnType<typeof Subscription.granting>>,
        catalog: (packageId: PackageId) => Promise<Catalog>,
        publisher: Publisher,
        context: MethodContext,
    ) {
        // set the account, its terms and how to read and write its lines
        this.accountId = accountId;
        this.granting = granting;
        this.#catalog = catalog;
        this.#publisher = publisher;
        this.#context = context;

        // start with no lines read and nothing rated
        this.#lines = new Map();
        this.#spent = new Map();
    }

    /** Start rating a page of an account's usage, reading its granting subscriptions once. */
    static async open(
        accountId: Identifier<"account">,
        catalog: (packageId: PackageId) => Promise<Catalog>,
        publisher: Publisher,
        context: MethodContext,
    ): Promise<Rating> {
        const granting = await Subscription.granting(accountId, context);

        return new Rating(accountId, granting, catalog, publisher, context);
    }

    /** Rate uses of SKUs into their resources' lines, returning the refused uses. */
    async rateSkus(uses: readonly Usage[]): Promise<CallError<string, unknown>[]> {
        // add each use to its line's tally, refusing one its terms reject
        const publishers = new Map<string, Promise<Seller>>();
        const tallies = new Map<string, Tally>();
        const refused: CallError<string, unknown>[] = [];
        for (const used of uses) {
            try {
                await this.#tally(used, publishers, tallies);
            } catch (error) {
                refused.push(Usage.refusal(error));
            }
        }

        // price each line's usage at the SKU's list price, capped by the spending limit
        const calls: Call[] = [];
        for (const [key, counted] of tallies) {
            const { sku, sold, terms, line } = counted;
            const conversion = Rating.#convert(sold, sku.definition.currency, terms.currency);
            const isSubscribed = terms.subscription !== undefined;
            const rated = Rate.usage(sku, counted.consumed, isSubscribed, conversion);
            const billedCost = await this.#cap(
                terms.subscription,
                line?.id ?? key,
                rated.billedCost,
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
                          accountId: this.accountId,
                          ...ObjectReference.columns(
                              "seller",
                              seller.reference(sold.scope, sold.id),
                          ),
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
        await this.#context.mutate(this.#context.self, calls);

        return refused;
    }

    /** Rate the period's usage of a seller's meter into the line of each metered price. */
    async rateMeter(meter: Meter, used: (period: Period) => Promise<number>): Promise<Call[]> {
        // take the granting subscriptions' items billing the meter
        const key = CatalogReference.key(meter.reference);
        const billing = this.granting.flatMap((row) =>
            row.items
                .filter(
                    ({ terms: { recurring } }) =>
                        recurring.usage === "metered" &&
                        CatalogReference.key(recurring.meter) === key,
                )
                .map((item) => ({ row, item })),
        );
        const calls: Call[] = [];
        for (const { row, item } of billing) {
            Rating.#requireRunning(row, this.#context.now);

            // find the price's line among the period's lines
            const period = { start: row.currentPeriodStart, end: row.currentPeriodEnd };
            const lines = await this.#periodLines(row);
            const line = Charge.requireOpen(lines.find((each) => each.priceId === item.priceId));

            // price the period's usage at the item's terms, at most what the spending limit leaves
            const quantity = await used(period);
            const listCost = Rate.price(item.terms, quantity);
            const terms = { row, includedUsage: SubscriptionItem.includedUsage(row.items) };
            const billedCost = await this.#cap(terms, line?.id ?? item.id, listCost);
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
                calls.push(charge.call("update", { id: line.id, ...costs }));
            } else {
                calls.push(
                    charge.call("create", {
                        accountId: this.accountId,
                        ...ObjectReference.columns("seller", ObjectReference.at("seller", row)),
                        chargeCategory: "Usage",
                        chargePeriodStart: period.start,
                        chargePeriodEnd: period.end,
                        serviceName: await this.#productName(item.priceId),
                        skuMeter: meter.name,
                        ...ObjectReference.columns("price", ObjectReference.at("price", item)),
                        consumedUnit: meter.definition.unit,
                        pricingUnit: meter.definition.unit,
                        billingCurrency: item.terms.currency,
                        ...costs,
                    }),
                );
            }
        }

        return calls;
    }

    /** Add a use of a SKU to the tally of its resource's line in the period. */
    async #tally(
        used: Usage,
        publishers: Map<string, Promise<Seller>>,
        tallies: Map<string, Tally>,
    ): Promise<void> {
        // read the SKU and its meter, and the reselling seller once per package
        const { sku: skuKey, meter: meterKey, quantity } = used.keys;
        const reference = CatalogReference.of(present(skuKey, "a use's SKU"));
        const metered = CatalogReference.of(meterKey);
        const sku = (await this.#catalog(reference.packageId)).sku(reference);
        const meter = (await this.#catalog(metered.packageId)).meter(metered);
        const publisher =
            publishers.get(sku.package.name) ??
            Seller.publisher(sku.package, this.#publisher, this.#context);
        publishers.set(sku.package.name, publisher);
        const sold = await publisher;

        // decide the terms at the use's time, refusing usage into an ended period
        const terms = this.#terms(sold, EventTime.milliseconds(used.time), sku);
        if (terms.subscription !== undefined) {
            Rating.#requireRunning(terms.subscription.row, this.#context.now);
        }

        // add the quantity to the line of the space or machine that used it, reading the line once
        const skuId = CatalogReference.key(reference);
        const resourceId = present(
            used.keys.spaceId ?? used.keys.machineId,
            `the space or machine of use ${used.id}`,
        );
        const key = [terms.period.start, resourceId, skuId].join(" ");
        const tally = tallies.get(key);
        if (tally !== undefined) {
            tally.consumed += quantity;
        } else {
            const line = await this.#line(terms.period.start, resourceId, skuId);
            const consumed =
                line === undefined ? 0 : present(line.consumedQuantity, "a line's quantity");
            tallies.set(key, {
                sku,
                meter,
                sold,
                terms,
                skuId,
                resourceId,
                line,
                consumed: consumed + quantity,
            });
        }
    }

    /** Read a resource's open usage line of a SKU. */
    async #line(
        periodStart: number,
        resourceId: string,
        skuId: string,
    ): Promise<Charge | undefined> {
        const [line] = await this.#context.read(charge).findMany({
            where: {
                accountId: this.accountId,
                chargePeriodStart: periodStart,
                resourceId,
                skuId,
                chargeCategory: "Usage",
            },
        });

        return Charge.requireOpen(line);
    }

    /** Read a subscription's open lines of its current period, once per page. */
    async #periodLines(row: Subscription): Promise<readonly Charge[]> {
        // reuse the lines read for the subscription already
        const known = this.#lines.get(row.id);
        if (known !== undefined) {
            return known;
        }

        // read them once
        const lines = await Charge.lines(row, row.currentPeriodStart, false, this.#context);
        this.#lines.set(row.id, lines);

        return lines;
    }

    /** Decide what a seller rates the account's usage at a time. */
    #terms(sold: Seller, time: number, sku: Sku): Terms {
        // find the account's granting subscription to the seller
        const chosen = this.granting.find((row) => row.sellerId === sold.id);

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
            throw new CallError("BAD_REQUEST", {
                message: `usage at ${time} falls before the current period of subscription ${chosen.id}, which billed it already`,
            });
        } else if (time >= chosen.currentPeriodEnd) {
            throw new CallError("SERVICE_UNAVAILABLE", {
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
    async #cap(billing: Terms["subscription"], key: string, billed: Cost): Promise<Cost> {
        // bill a line without a spending limit as rated
        if (billing === undefined || billing.row.spendingLimit === null) {
            return billed;
        }

        // take the period's billed costs once per subscription, as the page rates them
        const { row, includedUsage } = billing;
        const limit = billing.row.spendingLimit;
        const lines =
            this.#spent.get(row.id) ??
            new Map(
                (await this.#periodLines(row)).map((line) => [
                    line.id,
                    Cost.scaled(line.billedCost),
                ]),
            );
        this.#spent.set(row.id, lines);

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

    /** Read the name of the product a price belongs to. */
    async #productName(priceId: string): Promise<string> {
        // read the price, then its product
        const priced = await this.#context
            .read(price)
            .findFirst({ where: { id: price.identifier(priceId) } });
        const sold =
            priced === undefined
                ? undefined
                : await this.#context.read(product).findFirst({ where: { id: priced.parentId } });

        return Product.render(present(sold, `the product of price ${priceId}`)).name;
    }

    /** Convert a SKU's currency into a billing currency. */
    static #convert(sold: Seller, from: string, to: string): DecimalAmount {
        // take a SKU listed in the billing currency at its figure
        if (from === to) {
            return "1";
        }

        // price credits at the seller's price of a credit in the billing currency
        const credit = from === CREDITS ? sold.creditPrices[to] : undefined;
        if (credit === undefined) {
            throw new CallError("PRECONDITION_FAILED", {
                message: `seller ${sold.id} prices no ${from} in ${to}`,
            });
        }

        return credit;
    }

    /** Refuse usage into a subscription period that ended, so its close never races new lines. */
    static #requireRunning(row: Subscription, now: number): void {
        // refuse once the clock passed the period's end
        if (now >= row.currentPeriodEnd) {
            throw new CallError("SERVICE_UNAVAILABLE", {
                message: `the period of subscription ${row.id} ended, retry once the next one starts`,
            });
        }
    }
}
