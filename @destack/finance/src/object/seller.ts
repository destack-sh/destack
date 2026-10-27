import { through } from "@destack/access";
import { FinanceError } from "../error/error.ts";
import { account, space } from "@destack/account/object";
import { type Select, type Snapshot } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { type Package, PackageName } from "@destack/package";
import { Identifier, present, schema } from "@destack/schema";
import { Currency, DecimalAmount } from "../rate/amount.ts";
import { Country } from "@destack/locale";
import type {} from "@destack/package/import-meta";

/** An account as a merchant. */
export const seller = defineObject(
    {
        name: "seller",
        plural: "sellers",
        scope: space,
        owner: "account",
        fields: {
            /** The account the row belongs to. */
            account: field.reference(account).relate("account"),
            /** The payment provider. */
            provider: field.enum(["stripe"]).default("stripe"),
            /** The provider's identifier of the connected account, opened at onboarding. */
            providerId: field.string().optional(),
            /** The country the connected account does business from, fixed as it opens. */
            country: field.string(Country).optional(),
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
            connected: {
                kind: "check",
                condition: {
                    OR: [
                        { providerId: { isNull: true }, country: { isNull: true } },
                        { providerId: { isNotNull: true }, country: { isNotNull: true } },
                    ],
                },
            },
        },
    },
    (method) => ({
        create: method.create("sell", {
            fields: ["accountId", "provider", "creditPrices"],
        }),
        /** Open the provider's onboarding page for the seller's connected account. */
        onboard: method.mutation({
            permission: "sell",
            input: schema.object({
                /** The country the connected account does business from. */
                country: Country,
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
                /** The country the connected account opened in. */
                country: Country,
            }),
        }),
        /** Record the connected account's identifier, country and capabilities. */
        record: method.update("provide", {
            visibility: "package",
            fields: ["providerId", "country", "chargesEnabled", "payoutsEnabled"],
        }),
        /** Replace the seller's price book: the price of a credit in each currency it bills. */
        update: method.update("sell", { fields: ["creditPrices"] }),
    }),
);
/** A persisted seller. */
export type Seller = Select<typeof seller.table>;

/** Find the account selling a handle's packages to this home's buyers, none for a free handle. */
export type Publisher = (handle: string) => Promise<Identifier<"account"> | null>;

/** The accounts selling products. */
export const Seller = {
    /** Read the seller of a package here: the platform's for its handles, else the holder's. */
    async publisher(
        owner: Package,
        publisher: Publisher,
        reader: Pick<Snapshot, "read">,
    ): Promise<Seller> {
        // find the account selling the package's scope
        const { handle } = PackageName.split(owner.name);
        const accountId = await publisher(handle);

        // read its seller in this home
        const [found] =
            accountId === null
                ? []
                : await reader.read(seller.table).findMany({ where: { accountId } });
        if (found === undefined) {
            throw FinanceError.of(
                "SELLER_UNAVAILABLE",
                `${handle} has no offers for ${owner.name} here yet`,
            );
        }

        return found;
    },

    /** Require a seller's account to publish a package, by its scope or as the platform. */
    async requirePublisher(
        accountId: Identifier<"account">,
        owner: Package,
        publisher: Publisher,
        reader: Pick<Snapshot, "read">,
    ): Promise<void> {
        // refuse a package another account sells
        const { handle } = PackageName.split(owner.name);
        if ((await publisher(handle)) !== accountId) {
            const selling = await reader
                .read(account.table)
                .findFirst({ where: { id: accountId } });
            throw FinanceError.of(
                "PUBLISHER_MISMATCH",
                `account ${present(selling, `account ${accountId}`).handle} sells package ${owner.name}, which another account publishes`,
            );
        }
    },
};
