import { through } from "@destack/access";
import { account } from "@destack/account/object";
import {
    and,
    asc,
    check,
    eq,
    sql,
    uniqueIndex,
    type DatabaseConnection,
    type Select,
} from "@destack/db";
import { type CallOf, defineObject, field } from "@destack/object";
import { defineSchema, present, schema } from "@destack/schema";
import { Amount, Currency, DecimalAmount } from "../rate/amount.ts";
import { product } from "./product.ts";
import { type FeatureGrant, productFeature } from "./product-feature.ts";
import { ServiceError } from "@destack/service/error";
import { CatalogReference } from "../catalog/reference.ts";

/** The longest lookup key, one line of text. */
const LOOKUP_KEY_LENGTH = 200;

/** The fields of every recurring price beside its usage. */
const Recurrence = schema.object({
    /** The unit of the billing period. */
    interval: schema.enum(["day", "week", "month", "year"]),
    /** The intervals in one billing period. */
    intervalCount: schema.number().int().min(1),
});

/** How a recurring price bills: per period for a quantity, or for the usage a seller's meter counts. */
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

/** A price's terms, as subscription items and purchases keep them from their creation on. */
export const PriceTerms = defineSchema(
    schema.object({
        /** The currency of the amounts. */
        currency: Currency,
        /** The amount per unit of a per-unit price in whole minor units. */
        unitAmount: Amount.nullable(),
        /** The amount per unit of a per-unit price in fractions of minor units. */
        unitAmountDecimal: DecimalAmount.nullable(),
        /** The usage each billing period includes per unit, in minor units. */
        includedUsage: Amount,
        /** The billing period, and whether the price bills a quantity or metered usage. */
        recurring: Recurring,
        /** Whether the price charges per unit or by tiers. */
        billingScheme: schema.enum(["per_unit", "tiered"]),
        /** The tiers of a tiered price. */
        tiers: schema.array(Tier).nullable(),
    }),
);
/** A price's terms. */
export type PriceTerms = schema.Infer<typeof PriceTerms>;

/** What a product costs in one currency each billing period. */
export const price = defineObject({
    name: "price",
    plural: "prices",
    scope: account,
    nested: { in: product, delete: "cascade", receive: "sell" },
    fields: {
        /** The currency of the amounts. */
        currency: field.string(Currency),
        /** The amount per unit of a per-unit price in whole minor units. */
        unitAmount: field.integer().optional(),
        /** The amount per unit of a per-unit price in fractions of minor units, such as for bytes. */
        unitAmountDecimal: field.string(DecimalAmount).optional(),
        /** The usage each billing period includes per unit, in minor units of the currency. */
        includedUsage: field.integer().default(0),
        /** The billing period, and whether the price bills a quantity or metered usage. */
        recurring: field.json(Recurring),
        /** Whether the price charges per unit or by tiers. */
        billingScheme: field.enum(["per_unit", "tiered"]).default("per_unit"),
        /** The tiers of a tiered price, in rising order. */
        tiers: field.json(schema.array(Tier).min(1)).optional(),
        /** The key a seller looks the price up by. */
        lookupKey: field.string(schema.string().min(1).max(LOOKUP_KEY_LENGTH)).optional(),
        /** Whether buyers may buy at the price. */
        active: field.boolean().default(true),
        /** The payment provider's identifier of the price, mirrored at its creation. */
        providerId: field.string().optional(),
    },
    permissions: {
        read: through("parent", "read"),
        sell: through("parent", "sell"),
    },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("sell", {
            fields: [
                "currency",
                "unitAmount",
                "unitAmountDecimal",
                "includedUsage",
                "recurring",
                "billingScheme",
                "tiers",
                "lookupKey",
                "active",
            ],
            prepared: schema.object({
                /** The provider's identifier of the mirrored price, null without a provider. */
                price: schema.string().nullable(),
                /** The provider's identifier of the product, null without a provider. */
                product: schema.string().nullable(),
            }),
        }),
        update: method.update("sell", { fields: ["lookupKey", "active"] }),
    }),
    constraints: (entry) => [
        check(
            "price_billing_scheme",
            sql`(${entry.billingScheme} = 'per_unit' AND (${entry.unitAmount} IS NULL) <> (${entry.unitAmountDecimal} IS NULL) AND ${entry.tiers} IS NULL)
            OR (${entry.billingScheme} = 'tiered' AND ${entry.unitAmount} IS NULL AND ${entry.unitAmountDecimal} IS NULL AND ${entry.tiers} IS NOT NULL)`,
        ),
        check("price_unit_amount", sql`${entry.unitAmount} IS NULL OR ${entry.unitAmount} >= 0`),
        check("price_included_usage", sql`${entry.includedUsage} >= 0`),
        uniqueIndex("price_lookup_key")
            .on(entry.scope, entry.lookupKey)
            .where(sql`${entry.lookupKey} IS NOT NULL`),
    ],
});
/** A persisted price. */
export type Price = Select<typeof price.table>;

/** The prices buyers pay. */
export const Price = {
    /** Read the terms a price's creation sets, its fields' defaults filled in. */
    terms(input: CallOf<typeof price, "create">["input"]): PriceTerms {
        return {
            currency: input.currency,
            unitAmount: input.unitAmount ?? null,
            unitAmountDecimal: input.unitAmountDecimal ?? null,
            includedUsage: input.includedUsage ?? 0,
            recurring: input.recurring,
            billingScheme: input.billingScheme ?? "per_unit",
            tiers: input.tiers ?? null,
        };
    },

    /** Read an active price's terms and its product's grants. */
    async offer(
        database: DatabaseConnection,
        reference: { readonly scope: string; readonly id: string },
    ): Promise<{ readonly terms: PriceTerms; readonly grants: FeatureGrant[] }> {
        // read the price
        const scope = account.identifier(reference.scope);
        const id = price.identifier(reference.id);
        const [row] = await database
            .select()
            .from(price.table)
            .where(and(eq(price.table.scope, scope), eq(price.table.id, id)));
        if (row === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no price ${id}` });
        }

        // refuse a price or product taken off sale
        const [owner] = await database
            .select({ active: product.table.active })
            .from(product.table)
            .where(eq(product.table.id, row.parentId));
        if (!row.active) {
            throw new ServiceError("BAD_REQUEST", { message: `price ${id} is inactive` });
        } else if (!present(owner, "a price's product").active) {
            throw new ServiceError("BAD_REQUEST", {
                message: `product ${row.parentId} is inactive`,
            });
        }

        // read its product's grants
        const grants = await database
            .select({
                packageId: productFeature.table.packageId,
                feature: productFeature.table.feature,
                value: productFeature.table.value,
            })
            .from(productFeature.table)
            .where(eq(productFeature.table.parentId, row.parentId))
            .orderBy(asc(productFeature.table.packageId), asc(productFeature.table.feature));
        const {
            currency,
            unitAmount,
            unitAmountDecimal,
            includedUsage,
            recurring,
            billingScheme,
            tiers,
        } = row;

        return {
            terms: {
                currency,
                unitAmount,
                unitAmountDecimal,
                includedUsage,
                recurring,
                billingScheme,
                tiers,
            },
            grants,
        };
    },
};
