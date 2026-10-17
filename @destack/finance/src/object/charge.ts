import { account, space } from "@destack/account/object";
import type { Select, Snapshot } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { present, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { Cost, Currency, DecimalAmount } from "../rate/amount.ts";
import { SERVICE_CATEGORIES } from "../sku/sku.ts";
import { BILLING_PERMISSIONS } from "./billing.ts";
import { price } from "./price.ts";
import { seller } from "./seller.ts";
import { Subscription, SubscriptionItem } from "./subscription.ts";

/** How many uses one rating pass takes: enough for a busy account's pass to stay short. */
export const PAGE_EVENTS = 500;

/** What a charge is for, as FOCUS ChargeCategory has it: usage, or prepaid usage as credit. */
export const CHARGE_CATEGORIES = ["Usage", "Credit"] as const;

/** One priced line an account owes a seller for a billing period, shaped as a FOCUS row. */
export const charge = defineObject(
    {
        name: "charge",
        plural: "charges",
        scope: space,
        owner: "account",
        fields: {
            /** The account the row belongs to. */
            account: field.reference(account).relate("account"),
            /** The seller billing the line. */
            seller: field.object(seller),
            /** What the line is for, the FOCUS ChargeCategory. */
            chargeCategory: field.enum(CHARGE_CATEGORIES),
            /** When the billing period the line belongs to started, the FOCUS ChargePeriodStart. */
            chargePeriodStart: field.time(),
            /** When that period ends, the FOCUS ChargePeriodEnd. */
            chargePeriodEnd: field.time(),

            /** The FOCUS ServiceCategory of the SKU used, absent for a credit line. */
            serviceCategory: field.enum(SERVICE_CATEGORIES).optional(),
            /** The provider's service, the FOCUS ServiceName. */
            serviceName: field.string().optional(),
            /** The SKU used, by its package and name, the FOCUS SkuId. */
            skuId: field.string().optional(),
            /** The SKU's list price used, by the day it was read, the FOCUS SkuPriceId. */
            skuPriceId: field.string().optional(),
            /** The seller's metered price billing the usage, absent for a SKU's. */
            price: field.object(price).optional(),
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
                on: ["accountId", "chargePeriodStart", "priceId"],
                unique: true,
                where: { chargeCategory: "Usage", skuId: { isNull: true } },
            },
            credit: {
                kind: "index",
                on: ["accountId", "sellerId", "chargePeriodStart"],
                unique: true,
                where: { chargeCategory: "Credit" },
            },
            sku: { kind: "index", on: ["skuId", "status"] },
            period: {
                kind: "check",
                condition: { chargePeriodStart: { lt: { field: "chargePeriodEnd" } } },
            },
            // copy a usage line's quantities, unit and service onto its invoice line
            line: {
                kind: "check",
                condition: {
                    OR: [
                        { chargeCategory: { ne: "Usage" } },
                        {
                            consumedQuantity: { isNotNull: true },
                            pricingQuantity: { isNotNull: true },
                            pricingUnit: { isNotNull: true },
                            serviceName: { isNotNull: true },
                        },
                    ],
                },
            },
        },
    },
    (method) => ({
        create: method.create("provide", { isInternal: true }),
        rate: method.mutation({
            permission: "provide",
            target: "scope",
            isInternal: true,
            input: schema.object({
                /** The account whose usage is rated. */
                accountId: schema.identifier("account"),
                /** The log sequence the account's usage stands rated up to. */
                after: schema.number().int().nonnegative(),
            }),
            output: schema.object({
                /** The log sequence the rated page reaches. */
                sequence: schema.number().int().nonnegative(),
                /** The uses the page held. */
                uses: schema.number().int().nonnegative(),
            }),
        }),
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
    }),
);
/** A persisted charge. */
export type Charge = Select<typeof charge.table>;

/** The priced lines accounts owe sellers: their reads, and the invoice lines they read as. */
export const Charge = {
    /** Read the usage cost of a subscription's current period so far, in minor units. */
    async spend(row: Subscription, reader: Pick<Snapshot, "read">): Promise<number> {
        const lines = await Charge.lines(row, row.currentPeriodStart, false, reader);

        return Number(
            Cost.of(lines.reduce((total, line) => total + Cost.scaled(line.billedCost), 0n)),
        );
    },

    /** Read a subscription period's usage lines. */
    async lines(
        row: Subscription,
        periodStart: number,
        isOpen: boolean,
        reader: Pick<Snapshot, "read">,
    ): Promise<Charge[]> {
        // read the account's period lines, the seller's alone
        const rows = await reader.read(charge.table).findMany({
            where: {
                accountId: row.accountId,
                chargePeriodStart: periodStart,
                chargeCategory: "Usage",
                ...(isOpen ? { status: "open" } : {}),
            },
        });

        return rows.filter((line) => line.sellerId === row.sellerId);
    },

    /** Read a period's open lines in whole minor units, with the currency and included usage. */
    async draw(row: Subscription, periodStart: number, reader: Pick<Snapshot, "read">) {
        // round the lines to whole minor units adding up to their rounded total
        const lines = await Charge.lines(row, periodStart, true, reader);
        const amounts = Cost.apportion(lines.map((line) => Cost.scaled(line.billedCost)));
        const total = amounts.reduce((added, amount) => added + amount, 0);

        // read the currency and included usage from the subscription's items
        const [itemized] = await Subscription.itemized([row], reader);

        return { lines, amounts, total, ...SubscriptionItem.billing(present(itemized, row.id)) };
    },

    /** Describe a usage line as its invoice line reads: service, quantity and resource. */
    describe(line: Charge): string {
        // describe the service, the quantity in its unit and the resource when one used it
        const quantity = present(line.pricingQuantity, "a usage line's quantity");
        const unit = present(line.pricingUnit, "a usage line's unit");
        const resource = line.resourceId === null ? "" : ` for ${line.resourceId}`;
        const service = present(line.serviceName, "a usage line's service");

        return `${service} ${quantity.toPrecision(6)} ${unit}${resource}`;
    },

    /** Refuse a usage line its period's close already invoiced, as the next period takes over. */
    requireOpen(line: Charge | undefined): Charge | undefined {
        if (line !== undefined && line.status !== "open") {
            throw new CallError("SERVICE_UNAVAILABLE", {
                message: `the period of charge ${line.id} is closed, retry once the next one starts`,
            });
        }

        return line;
    },
};
