import { through } from "@destack/access";
import { space } from "@destack/account/object";
import { type Row, TABLE, type Select, type Snapshot, Table } from "@destack/db";
import { type MethodContext, defineObject, field } from "@destack/object";
import type { PackageId } from "@destack/package";
import { defineSchema, present, schema, Text } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import type { Catalog } from "../catalog/catalog.ts";
import { CatalogReference } from "../catalog/reference.ts";
import { Amount, Currency, DecimalAmount } from "../rate/amount.ts";
import { Rate } from "../rate/rate.ts";
import { product } from "./product.ts";
import { type FeatureGrant } from "../feature/grant.ts";
import { productFeature } from "./product-feature.ts";
import { seller } from "./seller.ts";

/** The fields of every recurring price beside its usage. */
const Recurrence = schema.object({
    /** The unit of the billing period. */
    interval: schema.enum(["day", "week", "month", "year"]),
    /** The intervals in one billing period. */
    intervalCount: schema.number().int().min(1),
});

/** How a recurring price bills: per period for a quantity, or for metered usage. */
export const Recurring = defineSchema(
    schema.discriminatedUnion("usage", [
        Recurrence.extend({
            /** A quantity billed each period. */
            usage: schema.literal("licensed"),
        }),
        Recurrence.extend({
            /** The usage a meter counts in each period. */
            usage: schema.literal("metered"),
            /** The meter counting the usage. */
            meter: CatalogReference,
        }),
    ]),
);

/** One tier of a tiered price: the amounts up to a quantity. */
export const Tier = defineSchema(
    schema.object({
        /** The highest quantity of the tier, inf for the last one. */
        upTo: schema.union([schema.number().int().min(1), schema.literal("inf")]),
        /** The amount per unit in the tier. */
        unitAmount: Amount.exactOptional(),
        /** The amount for the whole tier. */
        flatAmount: Amount.exactOptional(),
    }),
);

/** The fields of a price that rate its usage, which subscription items store as bought. */
export const PRICE_TERMS = {
    currency: true,
    unitAmount: true,
    unitAmountDecimal: true,
    includedUsage: true,
    recurring: true,
    billingScheme: true,
    tiers: true,
} as const;

/** What a product costs in one currency each billing period. */
export const price = defineObject(
    {
        name: "price",
        plural: "prices",
        scope: space,
        declarable: { from: "create" },
        nested: { in: product, delete: "cascade", receive: "sell" },
        fields: {
            /** The currency of the amounts. */
            currency: field.string(Currency),
            /** The amount per unit of a per-unit price in whole minor units. */
            unitAmount: field.integer().optional(),
            /** A per-unit price's amount per unit in fractional minor units, such as for bytes. */
            unitAmountDecimal: field.string(DecimalAmount).optional(),
            /** The usage each billing period includes per unit, in minor units of the currency. */
            includedUsage: field.integer().default(0),
            /** The billing period, and whether the price bills a quantity or metered usage. */
            recurring: field.json(Recurring),
            /** Whether the price charges per unit or by tiers. */
            billingScheme: field.enum(["per_unit", "tiered"]).default("per_unit"),
            /** The tiers of a tiered price, in rising order. */
            tiers: field.json(schema.array(Tier).min(1)).optional(),
            /** Whether buyers may buy at the price. */
            active: field.boolean().default(true),
            /** The payment provider's identifier of the price, mirrored at its first sale. */
            providerId: field.string().optional(),
        },
        permissions: {
            read: through("parent", "read"),
            sell: through("parent", "sell"),
            provide: through("parent", "provide"),
        },
        constraints: {
            billingScheme: {
                kind: "check",
                condition: {
                    OR: [
                        {
                            billingScheme: "per_unit",
                            tiers: { isNull: true },
                            OR: [
                                {
                                    unitAmount: { isNull: true },
                                    unitAmountDecimal: { isNotNull: true },
                                },
                                {
                                    unitAmount: { isNotNull: true },
                                    unitAmountDecimal: { isNull: true },
                                },
                            ],
                        },
                        {
                            billingScheme: "tiered",
                            unitAmount: { isNull: true },
                            unitAmountDecimal: { isNull: true },
                            tiers: { isNotNull: true },
                        },
                    ],
                },
            },
            unitAmount: {
                kind: "check",
                condition: { unitAmount: { OR: [{ isNull: true }, { gte: 0 }] } },
            },
            includedUsage: { kind: "check", condition: { includedUsage: { gte: 0 } } },
        },
    },
    (method) => ({
        create: method.create("sell", {
            fields: [
                "currency",
                "unitAmount",
                "unitAmountDecimal",
                "includedUsage",
                "recurring",
                "billingScheme",
                "tiers",
                "active",
            ],
        }),
        /** Record the provider's identifier of the price. */
        record: method.update("provide", { isInternal: true, fields: ["providerId"] }),
        update: method.update("sell", { fields: ["active"] }),
    }),
);
/** The provider's identifiers of the product and prices a sale mirrored first, by finance's own. */
export const PriceMirror = defineSchema(
    schema.object({
        /** The provider's identifier of the product, null where it was mirrored before. */
        product: schema.string().nullable(),
        /** The provider's identifier of each price mirrored, by the price's identifier. */
        prices: schema.record(schema.string(), schema.string()),
    }),
);
/** The provider's identifiers of the product and prices a sale mirrored first. */
export type PriceMirror = schema.Infer<typeof PriceMirror>;

/** A persisted price. */
export type Price = Select<typeof price.table>;

/** The prices buyers pay. */
export const Price = {
    /** Decide whether a price bills metered usage, which finance rates itself. */
    isMetered(terms: Pick<Price, "recurring">): boolean {
        return terms.recurring.usage === "metered";
    },

    /** Read a price's terms from its row, or from its creation input with field defaults. */
    terms(source: Row): Pick<Price, keyof typeof PRICE_TERMS> {
        const row = price.table[TABLE].defaults(source);
        const terms = Object.keys(PRICE_TERMS).map((key) => [key, row[key]]);

        return Table.selectSchema(price.table).pick(PRICE_TERMS).parse(Object.fromEntries(terms));
    },

    /** Read an active price's terms and its product's grants. */
    async offer(
        reference: { readonly scope: string; readonly id: string },
        reader: Pick<Snapshot, "read">,
    ): Promise<{
        readonly terms: Pick<Price, keyof typeof PRICE_TERMS>;
        readonly grants: FeatureGrant[];
    }> {
        // read the price where the reference says it lives
        const id = price.identifier(reference.id);
        const row = await reader.read(price.table).findFirst({ where: { id } });
        if (row === undefined || row.scope !== reference.scope) {
            throw new CallError("NOT_FOUND", { message: `no price ${id}` });
        }

        // refuse a price or product taken off sale
        const owner = await reader.read(product.table).findFirst({ where: { id: row.parentId } });
        if (!row.active) {
            throw new CallError("BAD_REQUEST", { message: `price ${id} is inactive` });
        } else if (!present(owner, "a price's product").active) {
            throw new CallError("BAD_REQUEST", {
                message: `product ${row.parentId} is inactive`,
            });
        }

        // read its product's grants, by package and feature
        const features = await reader
            .read(productFeature.table)
            .findMany({ where: { parentId: row.parentId } });
        const grants = features
            .map(({ packageId, feature, value }) => ({ packageId, feature, value }))
            .toSorted((left, right) =>
                left.packageId === right.packageId
                    ? Text.compare(left.feature, right.feature)
                    : Text.compare(left.packageId, right.packageId),
            );

        return { terms: Price.terms(row), grants };
    },

    /** Require a non-default product's price billing a declared meter and covering its usage. */
    async require(
        parentId: string,
        terms: Pick<Price, keyof typeof PRICE_TERMS>,
        reader: Pick<Snapshot, "read">,
        catalog: (packageId: PackageId) => Promise<Catalog>,
    ): Promise<void> {
        // refuse a price of a missing or default product
        const owner = await reader
            .read(product.table)
            .findFirst({ where: { id: product.identifier(parentId) } });
        if (owner === undefined) {
            throw new CallError("NOT_FOUND", { message: `no product ${parentId}` });
        } else if (owner.isDefault) {
            throw new CallError("BAD_REQUEST", {
                message: `product ${parentId} is every account's default, which refuses prices`,
            });
        }

        // refuse a currency without the seller's credit price
        const [sold] = await reader
            .read(seller.table)
            .findMany({ where: { accountId: owner.accountId } });
        if (sold === undefined) {
            throw new CallError("PRECONDITION_FAILED", {
                message: `account ${owner.accountId} sells nothing`,
            });
        } else if (sold.creditPrices[terms.currency] === undefined) {
            throw new CallError("BAD_REQUEST", {
                message: `seller ${sold.id} prices no credit in ${terms.currency}`,
            });
        }

        // refuse a meter no package declares
        const { recurring } = terms;
        if (recurring.usage === "metered") {
            (await catalog(recurring.meter.packageId)).meter(recurring.meter);
        }

        // refuse a fee below the usage it includes
        requireFee(terms);
    },

    /** Refuse prices on a product marked every account's default. */
    async requireNone(
        row: { readonly id: string; readonly isDefault?: boolean },
        context: Pick<MethodContext, "read">,
    ): Promise<void> {
        // refuse marking a product with prices as the default
        if (row.isDefault !== true) {
            return;
        }
        const priced = await context
            .read(price)
            .findMany({ where: { parentId: product.identifier(row.id) } });
        if (priced.length > 0) {
            throw new CallError("BAD_REQUEST", {
                message: `product ${row.id} has prices, which every account's default refuses`,
            });
        }
    },
};

/** Refuse a fee per period below its included usage, which would bill unpaid usage. */
function requireFee(terms: Pick<Price, keyof typeof PRICE_TERMS>): void {
    // read the fee of one unit of a licensed price
    if (terms.recurring.usage !== "licensed") {
        return;
    }
    const fee = Number(Rate.price(terms, 1));

    // refuse a fee below the included usage
    if (fee < terms.includedUsage) {
        throw new CallError("BAD_REQUEST", {
            message: `a price's included usage of ${terms.includedUsage} exceeds its fee of ${fee} ${terms.currency}`,
        });
    }
}
