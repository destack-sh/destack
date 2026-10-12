import { resource, through, union } from "@destack/access";
import { account, space } from "@destack/account/object";
import { type Select } from "@destack/db";
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
    scope: space,
    declarable: { from: "create" },
    owner: "account",
    fields: {
        /** The account the row belongs to. */
        account: field.reference(account).relate("account"),
        /** The name buyers see. */
        name: field.string(schema.string().min(1).max(NAME_LENGTH)),
        /** What buyers get. */
        description: field.string(schema.string().min(1).max(DESCRIPTION_LENGTH)),
        /** Whether buyers may buy it. */
        active: field.boolean().default(true),
        /** Whether accounts without a subscription get its grants, only valid without prices. */
        isDefault: field.boolean().default(false),
        /** The payment provider's identifier of the product, mirrored at its first sale. */
        providerId: field.string().optional(),
    },
    attributes: { active: "boolean" },
    permissions: {
        read: union(through("account", "read"), resource({ active: true })),
        sell: through("account", "manage"),
        provide: through("account", "serve"),
    },
    constraints: {
        default: { kind: "index", on: ["accountId"], unique: true, where: { isDefault: true } },
    },
}).methods((method) => ({
    get: method.get("read"),
    list: method.list("read"),
    create: method.create("sell", {
        fields: ["accountId", "name", "description", "active", "isDefault"],
    }),
    update: method.update("sell", {
        fields: ["name", "description", "active", "isDefault"],
    }),
    /** Record the provider's identifier of the product. */
    record: method.update("provide", { isInternal: true, fields: ["providerId"] }),
}));
/** A persisted product. */
export type Product = Select<typeof product.table>;
