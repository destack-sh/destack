import { through, union } from "@destack/access";
import { BILLING_PERMISSIONS } from "./billing.ts";
import { Country } from "@destack/locale";
import {
    account,
    entitlement,
    type Entitlement,
    space,
    type UsageState,
} from "@destack/account/object";
import { type Select, type Snapshot } from "@destack/db";
import { type IdentifierOf, type MethodContext, defineObject, field } from "@destack/object";
import type { PackageId } from "@destack/package";
import type { Call } from "@destack/sync";
import { CallError } from "@destack/runtime/error";
import { canonicalize, defineSchema, nth, present, schema, type Identifier } from "@destack/schema";
import { announcement } from "@destack/notification";
import { Catalog } from "../catalog/catalog.ts";
import { CatalogReference } from "../catalog/reference.ts";
import { Allotment, type Resolution } from "./allotment.ts";
import { Source } from "./source.ts";
import { Usage, usageLimit, type UsageSeries } from "./usage.ts";

/** The usage states the account's readers of billing hear of. */
const NOTIFIED: ReadonlySet<UsageState | null> = new Set(["near", "over", "blocked"]);

/** The input of an entitlement's creation: what it grants on a feature, without its account. */
type EntitlementInput = Pick<
    Entitlement,
    | "packageId"
    | "feature"
    | "kind"
    | "value"
    | "limit"
    | "usage"
    | "resetAt"
    | "state"
    | "resource"
>;

/** A derived entitlement input beside the usage state stored before. */
interface StateChange {
    /** The entitlement's input as derived now. */
    readonly input: EntitlementInput;
    /** The stored usage state, null for a new or unmetered entitlement. */
    readonly previous: UsageState | null;
    /** The start of the billing period ending first among the granting sources. */
    readonly periodStart: number;
}

/** The longest name or address line a provider stores. */
const LINE_LENGTH = 200;

/** A postal address invoices and taxes read. */
export const Address = defineSchema(
    schema.object({
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
    }),
);

/** A tax identifier of a customer, such as an EU VAT number. */
export const TaxId = defineSchema(
    schema.object({
        /** The identifier's type, such as eu_vat. */
        type: schema.string().regex(/^[a-z]{2}_[a-z_]+$/u),
        /** The identifier. */
        value: schema.string().min(1).max(LINE_LENGTH),
    }),
);

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
        /** Derive an account's entitlements from the products granting it features. */
        entitle: method.mutation({
            permission: "provide",
            target: "scope",
            isInternal: true,
            input: schema.object({
                /** The account entitled. */
                accountId: schema.identifier("account"),
            }),
            output: schema.object({}),
        }),
        /** Open the customer of an entitled account under the account's name. */
        open: method.create("provide", { isInternal: true, fields: ["accountId", "name"] }),
        /** Record the provider's identifier of the customer. */
        record: method.update("provide", { isInternal: true, fields: ["providerId"] }),
        update: method.update("bill", { fields: ["email", "name", "address", "taxIds"] }),
    }),
);
/** A persisted customer. */
export type Customer = Select<typeof customer.table>;

/** The accounts customers bill. */
export const Customer = {
    /** Derive an account's entitlements from its sources' grants, announcing new usage states. */
    async entitle(
        accountId: IdentifierOf<typeof account>,
        catalog: (packageId: PackageId) => Promise<Catalog>,
        context: MethodContext,
    ): Promise<void> {
        // write the entitlements the grants derive
        const events = Usage.series(context);
        const once = Catalog.once(catalog);
        const derivation = await diff(accountId, once, context, events, context.now);
        await context.mutate(context.self, derivation.calls);

        // announce each new usage state
        for (const { input, previous, periodStart } of derivation.states) {
            await announce({ ...input, accountId }, previous, periodStart, context);
        }
    },

    /** Decide whether an account's entitlements are current in ~1 ms, relisting 1M in ~17 min. */
    async isEntitlementCurrent(
        accountId: IdentifierOf<typeof account>,
        catalog: (packageId: PackageId) => Promise<Catalog>,
        reader: Pick<Snapshot, "read">,
        events: UsageSeries,
        now: number,
    ): Promise<boolean> {
        const once = Catalog.once(catalog);
        const derivation = await diff(accountId, once, reader, events, now);

        return derivation.calls.length === 0;
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
            throw new CallError("PRECONDITION_FAILED", {
                message: `account ${accountId} lacks a customer`,
            });
        }
    },
};

/** Announce a new usage state once per billing period, on its account's customer. */
async function announce(
    row: Pick<
        Entitlement,
        "accountId" | "packageId" | "feature" | "state" | "usage" | "limit" | "resetAt"
    >,
    previous: UsageState | null,
    periodStart: number,
    context: MethodContext,
): Promise<void> {
    // notify only a changed state the readers hear of
    if (row.state === previous || !NOTIFIED.has(row.state)) {
        return;
    }

    // skip a state announced this billing period
    const opened = await Customer.open(row.accountId, context);
    const feature = CatalogReference.key({ packageId: row.packageId, name: row.feature });
    const key = `${feature}:${row.state}:${periodStart}`;
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
            thread: `${feature}:${periodStart}`,
            audience: { kind: "permission", permission: "read", reason: "subscribed" },
            payload: {
                feature: row.feature,
                state: row.state,
                usage: row.usage,
                limit: row.limit,
            },
        }),
    ]);
}

/** Diff an account's stored entitlements against what its sources grant. */
async function diff(
    accountId: IdentifierOf<typeof account>,
    catalog: (packageId: PackageId) => Promise<Catalog>,
    reader: Pick<Snapshot, "read">,
    events: UsageSeries,
    now: number,
): Promise<{ readonly calls: Call[]; readonly states: readonly StateChange[] }> {
    // allot each source's grant of each feature of its product
    const allotments: Allotment[] = [];
    for (const source of await Source.list(accountId, now, reader)) {
        for (const grant of source.grants) {
            const allotment = await source.allot(accountId, grant, catalog, events, now);
            if (allotment !== undefined) {
                allotments.push(allotment);
            }
        }
    }

    // resolve each feature across its sources, with the platform resource it allows
    const derived = new Map<string, EntitlementInput>();
    const periods = new Map<string, number>();
    for (const granted of Map.groupBy(allotments, (allotment) => featureKey(allotment)).values()) {
        const { packageId, feature } = nth(granted, 0);
        const key = featureKey({ packageId, feature });
        const reference = { packageId, name: feature };
        const resource = (await catalog(packageId)).find(reference)?.definition.resource ?? null;
        periods.set(key, Allotment.periodStart(granted));
        derived.set(key, {
            packageId,
            feature,
            ...columns(Allotment.resolve(granted)),
            resource,
        });
    }

    // delete a stored one no grant derives, and update a changed one
    const stored = await reader.read(entitlement.table).findMany({ where: { accountId } });
    const changed = stored.flatMap((row): Call[] => {
        const wanted = derived.get(featureKey(row));
        if (wanted === undefined) {
            return [entitlement.call("delete", { id: row.id })];
        } else if (!isSame(row, wanted)) {
            const { value, limit, resetAt, state, resource } = wanted;

            return [
                entitlement.call("update", {
                    id: row.id,
                    value,
                    limit,
                    usage: wanted.usage,
                    resetAt,
                    state,
                    resource,
                }),
            ];
        }

        return [];
    });

    // create each derived entitlement not stored yet
    const previous = new Map(stored.map((row) => [featureKey(row), row.state]));
    const created = [...derived]
        .filter(([key]) => !previous.has(key))
        .map(([, wanted]) => entitlement.call("create", { ...wanted, accountId }));

    // list each derived entitlement's state beside its stored one
    const states = [...derived].map(([key, wanted]) => ({
        input: wanted,
        previous: previous.get(key) ?? null,
        periodStart: present(periods.get(key), "the feature's period"),
    }));

    return { calls: [...changed, ...created], states };
}

/** Lay a resolved grant out in an entitlement's columns. */
function columns(
    resolved: Resolution,
): Pick<Entitlement, "kind" | "value" | "limit" | "usage" | "resetAt" | "state"> {
    const unmetered = { limit: null, usage: null, resetAt: null, state: null };

    // copy a metered grant's limit, usage, reset and state
    if (resolved.kind === "metered") {
        return { ...resolved, value: null };
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

/** Key an entitlement or allotment by its feature. */
function featureKey(row: { readonly packageId: PackageId; readonly feature: string }): string {
    return CatalogReference.key({ packageId: row.packageId, name: row.feature });
}

/** Compare a stored entitlement with a derived input. */
function isSame(row: EntitlementInput, wanted: EntitlementInput): boolean {
    return (
        row.kind === wanted.kind &&
        canonicalize(row.value) === canonicalize(wanted.value) &&
        row.limit === wanted.limit &&
        row.usage === wanted.usage &&
        row.resetAt === wanted.resetAt &&
        row.state === wanted.state &&
        row.resource === wanted.resource
    );
}
