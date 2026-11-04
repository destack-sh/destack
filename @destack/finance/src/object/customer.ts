import { FinanceError } from "../error/error.ts";
import { through, union } from "@destack/access";
import { BILLING_PERMISSIONS } from "./billing.ts";
import { Country } from "@destack/locale";
import {
    account,
    entitlement,
    Account,
    type Allowance,
    Entitlement,
    space,
    type UsageState,
    USAGE_STATES,
} from "@destack/account/object";
import { type Select, type Snapshot, Table } from "@destack/db";
import { type IdentifierOf, type MethodContext, defineObject, field } from "@destack/object";
import type { Call } from "@destack/sync";
import { present, schema, type Identifier } from "@destack/schema";
import { announcement } from "@destack/notification";
import type { ReleaseReader } from "../release/reader.ts";
import { Allotment, type Resolution } from "./allotment.ts";
import { Source } from "./source.ts";
import { usageLimit, type UsageSeries } from "./usage.ts";
import type {} from "@destack/package/import-meta";

/** The usage states the account's readers of billing hear of. */
const NOTIFIED: ReadonlySet<UsageState> = new Set(["near", "over", "blocked"]);

/** A usage state of an entitlement that its account's readers of billing hear of, per period. */
const UsageAlert = Object.assign(
    Table.selectSchema(entitlement.table)
        .pick({ packageId: true, feature: true, limit: true })
        .extend({
            /** The usage of the feature since its last reset. */
            usage: schema.number().min(0),
            /** The usage state the readers hear of. */
            state: schema.enum(USAGE_STATES),
            /** The start of the billing period ending first among the granting sources. */
            periodStart: schema.number(),
        }),
    {
        /** Read the thread of an alert's announcements: its feature in its billing period. */
        thread(
            alert: Pick<Entitlement, "packageId" | "feature"> & { readonly periodStart: number },
        ): string {
            return `${Entitlement.key(alert)}:${alert.periodStart}`;
        },

        /** Read the key announcing an alert's state once in its thread. */
        key(
            alert: Pick<Entitlement, "packageId" | "feature"> & {
                readonly state: UsageState;
                readonly periodStart: number;
            },
        ): string {
            return `${Entitlement.key(alert)}:${alert.state}:${alert.periodStart}`;
        },
    },
);
/** A usage state of an entitlement that its account's readers of billing hear of. */
type UsageAlert = schema.Infer<typeof UsageAlert>;

/** The longest name or address line a provider stores. */
const LINE_LENGTH = 200;

/** A postal address invoices and taxes read. */
export const Address = schema.object({
    /** The street and number. */
    line1: schema.string().min(1).max(LINE_LENGTH),
    /** The apartment, suite or building. */
    line2: schema.string().min(1).max(LINE_LENGTH).exactOptional(),
    /** The city or town. */
    city: schema.string().min(1).max(LINE_LENGTH),
    /** The postal code. */
    postalCode: schema.string().min(1).max(LINE_LENGTH).exactOptional(),
    /** The state, province or region. */
    state: schema.string().min(1).max(LINE_LENGTH).exactOptional(),
    /** The country. */
    country: Country,
});

/** A tax identifier of a customer, such as an EU VAT number. */
export const TaxId = schema.object({
    /** The identifier's type, such as eu_vat. */
    type: schema.string().regex(/^[a-z]{2}_[a-z_]+$/u),
    /** The identifier. */
    value: schema.string().min(1).max(LINE_LENGTH),
});

/** An account as a buyer: the details its invoices show. */
export const customer = defineObject(
    {
        name: "customer",
        plural: "customers",
        scope: space,
        owner: "account",
        fields: {
            /** The account the row belongs to. */
            account: field.reference(account).relate("account"),
            /** The address invoices go to. */
            email: field.string(schema.email()).optional(),
            /** The name invoices show. */
            name: field.string(schema.string().min(1).max(LINE_LENGTH)),
            /** The billing address. */
            address: field.json(Address).optional(),
            /** The tax identifiers invoices show. */
            taxIds: field.json(schema.array(TaxId)).default([]),
            /** The provider's identifier of the customer. */
            providerId: field.string().optional(),
        },
        permissions: {
            ...BILLING_PERMISSIONS,
            // let its residency's servers read too, announcing the account's usage limits on it
            read: union(through("account", "read"), through("account", "serve")),
        },
        reserved: ["provide"],
        attachments: [announcement.attach({ by: "read" })],
        constraints: {
            account: { kind: "index", on: ["accountId"], unique: true },
            provider: {
                kind: "index",
                on: ["providerId"],
                unique: true,
                where: { providerId: { isNotNull: true } },
            },
        },
    },
    (method) => ({
        create: method.create("bill", {
            fields: ["accountId", "email", "name", "address", "taxIds"],
        }),
        /** Announce an account's usage alerts to its readers of billing, each once per period. */
        announce: method.mutation({
            permission: "provide",
            target: "scope",
            visibility: "package",
            input: schema.object({
                /** The account alerted. */
                accountId: schema.identifier("account"),
                /** The alerts of its derived entitlements. */
                alerts: schema.array(UsageAlert),
            }),
            output: schema.object({}),
        }),
        /** Open the customer of an entitled account under the account's name. */
        open: method.create("provide", { visibility: "package", fields: ["accountId", "name"] }),
        /** Record the provider's identifier of the customer. */
        record: method.update("provide", { visibility: "package", fields: ["providerId"] }),
        update: method.update("bill", { fields: ["email", "name", "address", "taxIds"] }),
    }),
);
/** A persisted customer. */
export type Customer = Select<typeof customer.table>;

/** The accounts customers bill. */
export const Customer = {
    /** Announce a usage alert on its account's customer, once per billing period. */
    async announce(
        accountId: IdentifierOf<typeof account>,
        alert: UsageAlert,
        context: MethodContext,
    ): Promise<void> {
        // skip a state announced this billing period
        const opened = await Customer.open(accountId, context);
        const key = UsageAlert.key(alert);
        const [announced] = await context.read(announcement).findMany({
            where: {
                packageId: usageLimit.reference.packageId,
                name: usageLimit.name,
                parentId: opened.id,
                key,
            },
        });
        if (announced !== undefined) {
            return;
        }

        // announce it to the account's readers of billing
        await context.mutate(context.caller, [
            usageLimit.call({
                source: customer.reference(opened.scope, opened.id),
                key,
                thread: UsageAlert.thread(alert),
                audience: { kind: "permission", permission: "read", reason: "subscribed" },
                payload: {
                    feature: alert.feature,
                    state: alert.state,
                    usage: alert.usage,
                    limit: alert.limit,
                },
            }),
        ]);
    },

    /** Resolve each feature an account's sources grant now, with its metered usage. */
    async resolve(
        accountId: IdentifierOf<typeof account>,
        releases: ReleaseReader,
        reader: Pick<Snapshot, "read">,
        events: UsageSeries,
        now: number,
    ): Promise<Resolution[]> {
        // allot each source's grant of each feature of its product
        const allotments: Allotment[] = [];
        for (const source of await Source.list(accountId, now, reader)) {
            for (const grant of source.grants) {
                const allotment = await source.allot(accountId, grant, releases, events, now);
                if (allotment !== undefined) {
                    allotments.push(allotment);
                }
            }
        }

        // resolve each feature across its sources
        const grouped = Map.groupBy(allotments, (allotment) => Entitlement.key(allotment));

        return [...grouped.values()].map((granted) => Allotment.resolve(granted));
    },

    /** Derive an account's entitlement changes and usage alerts from its features' resolutions. */
    async derive(
        accountId: IdentifierOf<typeof account>,
        resolutions: readonly Resolution[],
        reader: Pick<Snapshot, "read">,
    ): Promise<{ readonly calls: Call[]; readonly alerts: UsageAlert[] }> {
        // lay out each resolution as an allowance
        const allowances = resolutions.map((resolved): Allowance => {
            const { packageId, feature, resource } = resolved;

            return { packageId, feature, ...columns(resolved), resource };
        });

        // entitle the account at its home once its replicated entitlements differ, which the home compares again
        const stored = await reader
            .read(entitlement.table)
            .findMany({ where: { scope: accountId } });
        const replicated = new Map(stored.map((row) => [Entitlement.key(row), row]));
        const isOutdated =
            stored.length !== allowances.length ||
            allowances.some((wanted) => {
                const row = replicated.get(Entitlement.key(wanted));

                return row === undefined || !Entitlement.isSame(row, wanted);
            });
        const owner = await Account.of(reader, accountId);
        const calls = isOutdated
            ? [account.call("entitle", { ...account.target(owner), allowances })]
            : [];

        // alert each metered usage state the readers hear of, once it differs from the stored one
        const alerts = resolutions.flatMap((resolved): UsageAlert[] => {
            // keep a changed metered state the readers hear of
            const isChanged =
                replicated.get(Entitlement.key(resolved))?.state !== columns(resolved).state;
            if (resolved.kind !== "metered" || !isChanged || !NOTIFIED.has(resolved.state)) {
                return [];
            }
            const { packageId, feature, usage, limit, state, periodStart } = resolved;

            return [{ packageId, feature, usage, limit, state, periodStart }];
        });

        return { calls, alerts };
    },

    /** Read an account's customer, opening one under the account's name for an account without. */
    async open(accountId: Identifier<"account">, context: MethodContext): Promise<Customer> {
        // read the account's customer
        const [row] = await context.read(customer).findMany({ where: { accountId } });
        if (row !== undefined) {
            return row;
        }

        // open one under the account's name
        const owner = present(
            await context.read(account).findFirst({ where: { id: accountId } }),
            `the account ${accountId}`,
        );
        await context.mutate(context.self, [
            customer.call("open", { accountId, name: owner.name }),
        ]);
        const [opened] = await context.read(customer).findMany({ where: { accountId } });

        return present(opened, `the customer of account ${accountId}`);
    },

    /** Require an account's customer before the account subscribes or buys. */
    async require(
        accountId: Identifier<"account">,
        context: Pick<MethodContext, "read">,
    ): Promise<void> {
        const [row] = await context.read(customer).findMany({ where: { accountId } });
        if (row === undefined) {
            throw FinanceError.of("CUSTOMER_MISSING", `account ${accountId} has no customer`);
        }
    },
};

/** Lay a resolved grant out in an entitlement's columns. */
function columns(
    resolved: Resolution,
): Pick<Entitlement, "kind" | "value" | "limit" | "resetAt" | "state"> {
    const unmetered = { limit: null, resetAt: null, state: null };

    // copy a metered grant's limit, reset and state
    if (resolved.kind === "metered") {
        const { limit, resetAt, state } = resolved;

        return { kind: "metered", value: null, limit, resetAt, state };
    }
    // copy a static grant's value
    else if (resolved.kind === "static") {
        return { ...unmetered, kind: "static", value: resolved.value };
    }
    // grant access alone
    else {
        return { ...unmetered, kind: "boolean", value: null };
    }
}
