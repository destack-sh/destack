import { through } from "@destack/access";
import { account, space } from "@destack/account/object";
import { type Select, type Snapshot } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { type Package, PackageName } from "@destack/package";
import { Identifier, present, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Currency, DecimalAmount } from "../rate/amount.ts";
import { Country } from "./customer.ts";

/** An account as a merchant. */
export const seller = defineObject({
    name: "seller",
    plural: "sellers",
    scope: space,
    declarable: { from: "create" },
    owner: "account",
    fields: {
        /** The account the row belongs to. */
        account: field.reference(account).relate("account"),
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
        provide: through("account", "serve"),
    },
    reserved: ["provide"],
    constraints: {
        account: { kind: "index", on: ["accountId"], unique: true },
        provider: {
            kind: "index",
            on: ["providerId"],
            unique: true,
            where: { providerId: { isNotNull: true } },
        },
    },
}).methods((method) => ({
    get: method.get("read"),
    list: method.list("read"),
    create: method.create("sell", {
        fields: ["accountId", "provider", "country", "creditPrices"],
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
            /** The provider's identifier of the connected account, opened once. */
            providerId: schema.string(),
        }),
    }),
    /** Replace the seller's price book: the price of a credit in each currency it bills. */
    update: method.update("sell", { fields: ["creditPrices"] }),
    /** Record the provider's identifier and capabilities of the connected account. */
    record: method.update("provide", {
        isInternal: true,
        fields: ["providerId", "chargesEnabled", "payoutsEnabled"],
    }),
}));
/** A persisted seller. */
export type Seller = Select<typeof seller.table>;

/** Find the account selling a handle's packages to this home's buyers, none for a free handle. */
export type Publisher = (handle: string) => Promise<Identifier<"account"> | null>;

/** The accounts selling products. */
export const Seller = {
    /** Read the seller of a package here: the platform's for its handles, else the holder's. */
    async publisher(snapshot: Snapshot, owner: Package, publisher: Publisher): Promise<Seller> {
        // find the account selling the package's scope
        const { handle } = PackageName.split(owner.name);
        const accountId = await publisher(handle);

        // read its seller in this home, which a seller opens in each residency it sells to
        const [found] = accountId === null ? [] : await snapshot.rows(seller.table, { accountId });
        if (found === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `${handle} publishing ${owner.name} sells nothing here yet`,
            });
        }

        return found;
    },

    /** Require a seller's account to publish a package, holding its scope or being the platform. */
    async requirePublisher(
        snapshot: Snapshot,
        accountId: Identifier<"account">,
        owner: Package,
        publisher: Publisher,
    ): Promise<void> {
        // refuse a package another account sells
        const { handle } = PackageName.split(owner.name);
        if ((await publisher(handle)) !== accountId) {
            const selling = await snapshot.row(account.table, { id: accountId });
            throw new ServiceError("FORBIDDEN", {
                message: `account ${present(selling, `account ${accountId}`).handle} does not publish package ${owner.name}`,
            });
        }
    },
};
