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
import { defineObject, field } from "@destack/object";
import { defineSchema, present, schema } from "@destack/schema";
import { MeterReference } from "../meter/meter.ts";
import { type FeatureGrant, product, productFeature } from "./product.ts";
import { ServiceError } from "@destack/service/error";

/** The longest lookup key, one line of text. */
const LOOKUP_KEY_LENGTH = 200;

/** A currency by its ISO 4217 code, or a custom one such as x-credits. */
export const Currency = defineSchema(
    schema.string().regex(/^(?:[A-Z]{3}|x-[a-z0-9]+(?:-[a-z0-9]+)*)$(?![\s\S])/u),
);

/** An amount in the minor unit of its currency, such as cents. */
export const Amount = defineSchema(schema.number().int().min(0));

/** The fields of every recurring price beside its usage. */
const Recurrence = schema.object({
    /** The unit of the billing period. */
    interval: schema.enum(["day", "week", "month", "year"]),
    /** The intervals in one billing period. */
    intervalCount: schema.number().int().min(1),
});

/** How a recurring price bills: per period for a quantity, or for the usage a meter counts. */
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
            meter: MeterReference,
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
        /** The amount per unit of a per-unit price. */
        unitAmount: Amount.nullable(),
        /** Whether the price charges once or recurs. */
        type: schema.enum(["one_time", "recurring"]),
        /** The billing period and usage of a recurring price. */
        recurring: Recurring.nullable(),
        /** Whether the price charges per unit or by tiers. */
        billingScheme: schema.enum(["per_unit", "tiered"]),
        /** The tiers of a tiered price. */
        tiers: schema.array(Tier).nullable(),
    }),
);
/** A price's terms. */
export type PriceTerms = schema.Infer<typeof PriceTerms>;

/** What a product costs in one currency: once, or each billing period. */
export const price = defineObject({
    name: "price",
    plural: "prices",
    scope: account,
    nested: { in: product, delete: "cascade", receive: "sell" },
    fields: {
        /** The currency of the amounts. */
        currency: field.string(Currency),
        /** The amount per unit of a per-unit price. */
        unitAmount: field.integer().optional(),
        /** Whether the price charges once or recurs. */
        type: field.enum(["one_time", "recurring"]),
        /** The billing period and usage of a recurring price. */
        recurring: field.json(Recurring).optional(),
        /** Whether the price charges per unit or by tiers. */
        billingScheme: field.enum(["per_unit", "tiered"]).default("per_unit"),
        /** The tiers of a tiered price, in rising order. */
        tiers: field.json(schema.array(Tier).min(1)).optional(),
        /** The key a seller looks the price up by. */
        lookupKey: field.string(schema.string().min(1).max(LOOKUP_KEY_LENGTH)).optional(),
        /** Whether buyers may buy at the price. */
        active: field.boolean().default(true),
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
                "type",
                "recurring",
                "billingScheme",
                "tiers",
                "lookupKey",
                "active",
            ],
        }),
        update: method.update("sell", { fields: ["lookupKey", "active"] }),
    }),
    constraints: (entry) => [
        check("price_type", sql`(${entry.type} = 'recurring') = (${entry.recurring} IS NOT NULL)`),
        check(
            "price_billing_scheme",
            sql`(${entry.billingScheme} = 'per_unit' AND ${entry.unitAmount} IS NOT NULL AND ${entry.tiers} IS NULL)
            OR (${entry.billingScheme} = 'tiered' AND ${entry.unitAmount} IS NULL AND ${entry.tiers} IS NOT NULL)`,
        ),
        check("price_unit_amount", sql`${entry.unitAmount} IS NULL OR ${entry.unitAmount} >= 0`),
        uniqueIndex("price_lookup_key")
            .on(entry.scope, entry.lookupKey)
            .where(sql`${entry.lookupKey} IS NOT NULL`),
    ],
});
/** A persisted price. */
export type Price = Select<typeof price.table>;

/** The prices buyers pay. */
export const Price = {
    /** Read an active price's terms and its active product's feature grants, which a subscription item or purchase keeps. */
    async snapshot(
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
        const { currency, unitAmount, type, recurring, billingScheme, tiers } = row;

        return { terms: { currency, unitAmount, type, recurring, billingScheme, tiers }, grants };
    },
};
