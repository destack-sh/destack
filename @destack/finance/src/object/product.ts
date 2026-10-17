import { resource, through, union } from "@destack/access";
import { account, space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { Localization, SOURCE_LOCALE } from "@destack/locale";
import { defineObject, field } from "@destack/object";

/** The source language, in which providers without translations show products. */
const SOURCE = Localization.of(SOURCE_LOCALE, []);

/** A good or service a seller offers. */
export const product = defineObject(
    {
        name: "product",
        plural: "products",
        scope: space,
        declarable: { from: "create" },
        owner: "account",
        fields: {
            /** The account the row belongs to. */
            account: field.reference(account).relate("account"),
            /** The name buyers see, rendered with the declaring package's catalogs. */
            name: field.message(),
            /** What buyers get, rendered with the declaring package's catalogs. */
            description: field.message(),
            /** Whether buyers may buy it. */
            active: field.boolean().default(true),
            /** Whether accounts without a subscription get its grants, valid without prices. */
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
    },
    (method) => ({
        create: method.create("sell", {
            fields: ["accountId", "name", "description", "active", "isDefault"],
        }),
        /** Record the provider's identifier of the product. */
        record: method.update("provide", { isInternal: true, fields: ["providerId"] }),
        update: method.update("sell", {
            fields: ["name", "description", "active", "isDefault"],
        }),
    }),
);
/** A persisted product. */
export type Product = Select<typeof product.table>;

/** Products as providers without translations show them. */
export const Product = {
    /** Render a product's name and description in the source language. */
    render(row: Pick<Product, "name" | "description">): { name: string; description: string } {
        return { name: SOURCE.render(row.name), description: SOURCE.render(row.description) };
    },
};
