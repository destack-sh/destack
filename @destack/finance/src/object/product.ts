import { resource, through, union } from "@destack/access";
import { account } from "@destack/account/object";
import { sql, uniqueIndex, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** The longest product name, one line of text. */
const NAME_LENGTH = 250;

/** The longest product description, a few paragraphs. */
const DESCRIPTION_LENGTH = 4000;

/** A good or service a seller offers. */
export const product = defineObject({
    name: "product",
    plural: "products",
    scope: account,
    fields: {
        /** The name buyers see. */
        name: field.string(schema.string().min(1).max(NAME_LENGTH)),
        /** What buyers get. */
        description: field.string(schema.string().min(1).max(DESCRIPTION_LENGTH)),
        /** Whether buyers may buy it. */
        active: field.boolean().default(true),
        /** Whether accounts without a subscription to the seller get its grants, which a product without prices alone may be. */
        isDefault: field.boolean().default(false),
        /** The payment provider's identifier of the product, mirrored with its first price. */
        providerId: field.string().optional(),
    },
    attributes: { active: "boolean" },
    permissions: {
        read: union(through("account", "read"), resource({ active: true })),
        sell: through("account", "manage"),
    },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("sell", {
            fields: ["name", "description", "active", "isDefault"],
        }),
        update: method.update("sell", {
            fields: ["name", "description", "active", "isDefault"],
        }),
        /** Record the provider's identifier of the product. */
        record: method.update(null, { isSystem: true, fields: ["providerId"] }),
    }),
    constraints: (entry) => [
        uniqueIndex("product_default")
            .on(entry.scope)
            .where(sql`${entry.isDefault}`),
    ],
});
/** A persisted product. */
export type Product = Select<typeof product.table>;
