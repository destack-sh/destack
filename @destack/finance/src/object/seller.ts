import { none, through } from "@destack/access";
import { account } from "@destack/account/object";
import { eq, uniqueIndex, type DatabaseConnection, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import type { Package } from "@destack/package";
import type { Identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Country } from "./customer.ts";

/** An account as a merchant: its connected account at the payment provider, which products and charges go through. */
export const seller = defineObject({
    name: "seller",
    plural: "sellers",
    scope: account,
    fields: {
        /** The payment provider. */
        provider: field.enum(["stripe"]).default("stripe"),
        /** The provider's identifier of the connected account. */
        providerId: field.string().optional(),
        /** Where the provider's onboarding of the account stands. */
        status: field.state({
            initial: "onboarding",
            transitions: {
                activate: {
                    from: ["onboarding", "restricted"],
                    to: "active",
                    permission: "provide",
                },
                restrict: {
                    from: ["onboarding", "active"],
                    to: "restricted",
                    permission: "provide",
                },
            },
        }),
        /** Whether the provider lets the account take charges. */
        chargesEnabled: field.boolean().default(false),
        /** Whether the provider pays the account out. */
        payoutsEnabled: field.boolean().default(false),
        /** The country the account does business from. */
        country: field.string(Country),
    },
    permissions: {
        read: through("account", "read"),
        sell: through("account", "manage"),
        provide: none(),
    },
    reserved: ["provide"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("sell", { fields: ["provider", "country"] }),
        /** Record the provider's identifier and capabilities of the connected account. */
        record: method.update(null, {
            isSystem: true,
            fields: ["providerId", "chargesEnabled", "payoutsEnabled"],
        }),
    }),
    constraints: (entry) => [uniqueIndex("seller_account").on(entry.scope)],
});
/** A persisted seller. */
export type Seller = Select<typeof seller.table>;

/** The accounts selling products. */
export const Seller = {
    /** Require a seller's account to publish a package, as the handle scoping its name: `@<handle>/<name>`. */
    async requirePublisher(
        database: DatabaseConnection,
        accountId: Identifier<"account">,
        owner: Package,
    ): Promise<void> {
        // read the seller's handle from its account's copy
        const [copied] = await database
            .select({ handle: account.table.handle })
            .from(account.table)
            .where(eq(account.table.id, accountId));
        if (copied === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no account ${accountId}` });
        }

        // refuse a package another account publishes
        const handle = owner.name.slice(1, owner.name.indexOf("/"));
        if (handle !== copied.handle) {
            throw new ServiceError("FORBIDDEN", {
                message: `account ${copied.handle} does not publish package ${owner.name}`,
            });
        }
    },
};
