import { FinanceError } from "../error/error.ts";
import { through, union } from "@destack/access";
import { BILLING_PERMISSIONS } from "./billing.ts";
import { argument, Country, plural, t } from "@destack/locale";
import { account, space } from "@destack/account/object";
import { type Allowance, type UsageState, USAGE_STATES } from "../feature/allowance.ts";
import { quota, type Quota } from "./quota.ts";
import { type Select, Table } from "@destack/db";
import type { Snapshot } from "@destack/db/database";
import { type IdentifierOf, type MethodContext, defineObject, field } from "@destack/object";
import type { Call } from "@destack/sync";
import { present, schema, type Identifier } from "@destack/schema";
import { activity, announcement } from "@destack/notification/object";
import type { ReleaseReader } from "../release/reader.ts";
import { Allotment, type Resolution } from "./allotment.ts";
import { Grant } from "./grant.ts";
import { usageLimit, type UsageSeries } from "./usage.ts";
import type {} from "@destack/package/import-meta";

/** The usage states the account's readers of billing hear of. */
const NOTIFIED: ReadonlySet<UsageState> = new Set(["near", "over", "blocked"]);

/** A usage state of a quota that its account's readers of billing hear of, per period. */
const UsageAlert = Object.assign(
    Table.selectSchema(quota.table)
        .pick({ packageId: true, feature: true, limit: true })
        .extend({
            /** The usage of the feature since its last reset. */
            usage: schema.number().min(0),
            /** The usage state the readers hear of. */
            state: schema.enum(USAGE_STATES),
            /** The start of the billing period ending first among the grants granting it. */
            periodStart: schema.number(),
        }),
    {
        /** Read the thread of an alert's announcements: its feature in its billing period. */
        thread(
            alert: Pick<Allowance, "packageId" | "feature"> & { readonly periodStart: number },
        ): string {
            return `${featureKey(alert)}:${alert.periodStart}`;
        },

        /** Read the key announcing an alert's state once in its thread. */
        key(
            alert: Pick<Allowance, "packageId" | "feature"> & {
                readonly state: UsageState;
                readonly periodStart: number;
            },
        ): string {
            return `${featureKey(alert)}:${alert.state}:${alert.periodStart}`;
        },
    },
);
/** A usage state of a quota that its account's readers of billing hear of. */
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
        title: t`${plural(argument("count"), { one: "Customer", other: "Customers" })}`,
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
        attachments: [activity.attach({ by: "read" }), announcement.attach({ by: "read" })],
        constraints: {
            account: { kind: "index", on: ["accountId"], unique: true },
            provider: {
                kind: "index",
                on: ["providerId"],
                unique: true,
                where: { providerId: { isNotNull: true } },
            },
        },
        access: {
            permissions: {
                ...BILLING_PERMISSIONS,
                // let its region's servers read too, announcing the account's usage limits on it
                read: union(
                    through("account", "read"),
                    through("account", "serve"),
                    through("account", "sell"),
                ),
            },
            reserved: ["provide"],
        },
    },
    (method) => ({
        create: method.create("bill", {
            fields: ["accountId", "email", "name", "address", "taxIds"],
        }),
        /** Announce an account's usage alerts to its readers of billing, each once per period. */
        announce: method.mutation({
            title: t.context("action")`Announce`,
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

    /** Resolve each feature an account's grants allow now, with its metered usage. */
    async resolve(
        accountId: IdentifierOf<typeof account>,
        releases: ReleaseReader,
        reader: Pick<Snapshot, "read">,
        events: UsageSeries,
        now: number,
    ): Promise<Resolution[]> {
        // allot each feature of each grant
        const allotments: Allotment[] = [];
        for (const grant of await Grant.list(accountId, now, reader)) {
            const budget = await grant.budget(reader);
            for (const feature of grant.features) {
                const allotment = await grant.allot(
                    accountId,
                    feature,
                    releases,
                    events,
                    now,
                    budget,
                );
                if (allotment !== undefined) {
                    allotments.push(allotment);
                }
            }
        }

        // resolve each feature across its grants
        const grouped = Map.groupBy(allotments, (allotment) => featureKey(allotment));

        return [...grouped.values()].map((granted) => Allotment.resolve(granted));
    },

    /** Derive an account's quota changes and usage alerts from its metered resolutions. */
    async derive(
        accountId: IdentifierOf<typeof account>,
        resolutions: readonly Resolution[],
        reader: Pick<Snapshot, "read">,
        home: string,
    ): Promise<{ readonly calls: Call[]; readonly alerts: UsageAlert[] }> {
        // compare each metered resolution with the account's stored quota of its feature
        const meters = resolutions.flatMap((resolved) =>
            resolved.kind === "metered" ? [resolved] : [],
        );
        const stored = await reader.read(quota.table).findMany({ where: { accountId } });
        const quotas = new Map(stored.map((row) => [featureKey(row), row]));
        const calls: Call[] = [];
        for (const resolved of meters) {
            const { packageId, feature, limit, usage, resetAt, state, periodStart } = resolved;
            const terms = { limit, usage, resetAt, state, periodStart };
            const row = quotas.get(featureKey(resolved));
            if (row === undefined) {
                calls.push(
                    quota.call("create", {
                        ...quota.in(home),
                        accountId,
                        packageId,
                        feature,
                        ...terms,
                    }),
                );
            } else if (!isSameQuota(row, terms)) {
                calls.push(quota.call("update", { ...quota.target(row), ...terms }));
            }
        }

        // delete the quotas of features no grant meters any more
        const metered = new Set(meters.map((resolved) => featureKey(resolved)));
        for (const row of stored) {
            if (!metered.has(featureKey(row))) {
                calls.push(quota.call("delete", quota.target(row)));
            }
        }

        // alert each changed usage state the readers hear of
        const alerts = meters.flatMap((resolved): UsageAlert[] => {
            // keep a changed state the readers hear of
            const isChanged = quotas.get(featureKey(resolved))?.state !== resolved.state;
            if (!isChanged || !NOTIFIED.has(resolved.state)) {
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
            throw FinanceError.of("CUSTOMER_BLOCKED", "cannot bill: this account has no customer");
        }
    },
};

/** Key a feature's grant, resolution or quota by its package and name. */
function featureKey(row: Pick<Allowance, "packageId" | "feature">): string {
    return `${row.packageId}/${row.feature}`;
}

/** Report whether a stored quota already holds a resolution's terms. */
function isSameQuota(
    row: Quota,
    terms: Pick<Quota, "limit" | "usage" | "resetAt" | "state" | "periodStart">,
): boolean {
    return (
        row.limit === terms.limit &&
        row.usage === terms.usage &&
        row.resetAt === terms.resetAt &&
        row.state === terms.state &&
        row.periodStart === terms.periodStart
    );
}
