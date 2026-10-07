import { none, through } from "@destack/access";
import { account } from "@destack/account/object";
import { eq, uniqueIndex, type DatabaseConnection, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import type { Package } from "@destack/package";
import { schema, type Identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Currency, DecimalAmount } from "../rate/amount.ts";
import { Country } from "./customer.ts";

/** An account as a merchant. */
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
        /** The price of a credit in each currency the seller bills. */
        creditPrices: field.json(schema.record(Currency, DecimalAmount)).default({}),
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
        create: method.create("sell", {
            fields: ["provider", "country", "creditPrices"],
            prepared: schema.object({
                /** The provider's identifier of the opened connected account, null without a provider. */
                providerId: schema.string().nullable(),
            }),
        }),
        /** Open the provider's onboarding page for the seller's connected account. */
        onboard: method.mutation({
            permission: "sell",
            input: schema.object({
                /** The page to return to once onboarding ends. */
                returnUrl: schema.url(),
                /** The page to return to when the onboarding link expired. */
                refreshUrl: schema.url(),
            }),
            output: schema.object({
                /** The onboarding page. */
                url: schema.url(),
            }),
            prepared: schema.object({
                /** The onboarding page. */
                url: schema.url(),
            }),
        }),
        /** Replace the seller's price book: the price of a credit in each currency it bills. */
        update: method.update("sell", { fields: ["creditPrices"] }),
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
    /** Read the seller publishing a package. */
    async publisher(database: DatabaseConnection, owner: Package): Promise<Seller> {
        // find the seller of the account holding the package's scope as its handle
        const handle = owner.name.slice(1, owner.name.indexOf("/"));
        const [row] = await database
            .select({ seller: seller.table })
            .from(seller.table)
            .innerJoin(account.table, eq(account.table.id, seller.table.scope))
            .where(eq(account.table.handle, handle));
        if (row === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `account ${handle} publishing ${owner.name} sells nothing here yet`,
            });
        }

        return row.seller;
    },

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
