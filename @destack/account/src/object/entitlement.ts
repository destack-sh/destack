import { through, union } from "@destack/access";
import { type DatabaseConnection, type Select, Snapshot, Transaction } from "@destack/db";
import {
    type MethodContext,
    type Condition,
    defineObject,
    defineShape,
    field,
    ObjectWatch,
} from "@destack/object";
import { PackageId } from "@destack/package";
import type { FeatureReference } from "@destack/object";
import { type Identifier, present, schema } from "@destack/schema";
import { Replica, Scope } from "@destack/sync";
import { Account, account } from "./account.ts";
import { space, Space, type SpaceResource } from "./space.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** The platform resources a feature allows, which machines and spaces enforce. */
export const ENTITLEMENT_RESOURCES = [
    "database",
    "storage",
    "compute",
    "network",
    "tenancy",
    "domain",
] as const;

/** A platform resource a feature allows. */
export type EntitlementResource = (typeof ENTITLEMENT_RESOURCES)[number];

/** What a feature grants: access alone, a fixed value, or usage of meters up to a limit. */
export const FEATURE_KINDS = ["boolean", "static", "metered"] as const;

/** Where a space's work runs: shared or dedicated cloud machines, or its people's computers. */
export const Tenancy = schema.enum(["shared", "dedicated", "local"]);
/** Where a space's work runs. */
export type Tenancy = schema.Infer<typeof Tenancy>;

/** Where a metered usage stands against its limit and its subscription's budget. */
export const USAGE_STATES = ["within", "near", "over", "blocked"] as const;

/** Where a metered usage stands. */
export type UsageState = (typeof USAGE_STATES)[number];

/** The name of the shape replicating one account's entitlements. */
const ENTITLEMENT_SHAPE = "entitlement";

/** What an account may use of a feature across every product granting it. */
export const entitlement = defineObject(
    {
        name: "entitlement",
        plural: "entitlements",
        scope: account,
        fields: {
            /** The package declaring the feature. */
            packageId: field.string(PackageId),
            /** The feature's name in its package. */
            feature: field.string(schema.string().min(1)),
            /** What the feature grants. */
            kind: field.enum(FEATURE_KINDS),
            /** The value of a static feature. */
            value: field.json(schema.json()).optional(),
            /** The usage limit of a metered feature, unlimited without one. */
            limit: field.integer().optional(),
            /** When a metered feature's usage resets, absent for usage that never resets. */
            resetAt: field.time().optional(),
            /** Where the usage of a metered feature stands against its limit and budget. */
            state: field.enum(USAGE_STATES).optional(),
            /** The platform resource the feature allows, absent for a package's feature. */
            resource: field.enum(ENTITLEMENT_RESOURCES).optional(),
        },
        permissions: {
            // let the account's readers and its residency's servers read
            read: union(through("scope", "read"), through("scope", "serve")),
            // let its residency's servers derive it
            provide: through("scope", "serve"),
        },
        reserved: ["provide"],
        constraints: {
            feature: { kind: "index", on: ["scope", "packageId", "feature"], unique: true },
            resource: {
                kind: "index",
                on: ["scope", "resource"],
                unique: true,
                where: { resource: { isNotNull: true } },
            },
            kind: {
                kind: "check",
                condition: {
                    AND: [
                        {
                            OR: [
                                { kind: "metered", state: { isNotNull: true } },
                                { kind: { ne: "metered" }, state: { isNull: true } },
                            ],
                        },
                        {
                            OR: [
                                { kind: "metered" },
                                { limit: { isNull: true }, resetAt: { isNull: true } },
                            ],
                        },
                        { OR: [{ kind: "static" }, { value: { isNull: true } }] },
                    ],
                },
            },
        },
    },
    (method) => ({
        create: method.create("provide", { isInternal: true }),
        update: method.update("provide", {
            isInternal: true,
            fields: ["value", "limit", "resetAt", "state", "resource"],
        }),
        delete: method.delete("provide", { isInternal: true }),
    }),
);
/** An account's entitlement to a feature, as its table stores it. */
export type Entitlement = Select<typeof entitlement.table>;

/** The parameters of a replica of one account's entitlements. */
const EntitlementParameters = schema.object({
    /** The account whose entitlements are replicated. */
    account: schema.identifier("account"),
});

/** The shape replicating one account's entitlements from the universe, their home. */
export const entitlementShape = defineShape({
    name: ENTITLEMENT_SHAPE,
    parameters: EntitlementParameters,
    objects: [entitlement],
    queries: ({ account: accountId }) => [
        entitlement.query({ scopes: "every", where: { scope: accountId } }),
    ],
});

/** The condition of a use before an account's entitlements finish replicating: not yet known. */
const PENDING: Condition = {
    status: "unknown",
    reason: "Pending",
    message: "waiting for the account's plan",
};

/** The fields of an entitlement that decide a use. */
export type EntitlementTerms = Pick<
    Entitlement,
    "packageId" | "feature" | "resource" | "value" | "limit" | "state"
>;

/** A use a machine checks against an account's entitlements. */
export type EntitlementUse =
    | SpaceResource
    | { readonly tenancy: Tenancy }
    | { readonly feature: FeatureReference };

/** Account entitlements as their followers replicate and enforce them. */
export const Entitlement = {
    /** Read an account's replicated entitlement to a resource or feature, absent without a plan. */
    async read(
        reader: Pick<Snapshot, "read">,
        accountId: Identifier<"account">,
        use: EntitlementResource | EntitlementUse,
    ): Promise<Entitlement | undefined> {
        const rows = await reader.read(entitlement.table).findMany({ where: { scope: accountId } });

        return rows.find((row) => Entitlement.matches(row, use));
    },

    /** Report whether an entitlement grants a use: its resource, its tenancy or its feature. */
    matches(
        entry: Pick<EntitlementTerms, "resource" | "packageId" | "feature">,
        use: EntitlementResource | EntitlementUse,
    ): boolean {
        // match a resource by name
        if (typeof use === "string") {
            return entry.resource === use;
        }
        // match a tenancy by its resource
        else if ("tenancy" in use) {
            return entry.resource === "tenancy";
        }
        // match a feature by package and name
        else {
            return entry.packageId === use.feature.packageId && entry.feature === use.feature.name;
        }
    },

    /** Read an account's replicated entitlements, absent until the replica first completes. */
    async replicated(
        database: DatabaseConnection,
        accountId: Identifier<"account">,
    ): Promise<readonly Entitlement[] | undefined> {
        // wait for the replica's first complete snapshot of the account's entitlements, wherever it lives
        const [replica] = await entitlementShape.replicas(database, { account: accountId });
        if (
            replica === undefined ||
            !(await Replica.isSynced(database, replica.name, replica.scope))
        ) {
            return undefined;
        }

        return Snapshot.live(database)
            .read(entitlement.table)
            .findMany({ where: { scope: accountId } });
    },

    /** Decide the condition of a use from an account's replicated entitlements. */
    decide(rows: readonly EntitlementTerms[] | undefined, use: EntitlementUse): Condition {
        // wait while the entitlements replicate
        if (rows === undefined) {
            return PENDING;
        }

        // find the entitlement to the resource or feature
        const row = rows.find((entry) => Entitlement.matches(entry, use));
        const used = Entitlement.describe(use);

        // refuse a use no plan grants, and a tenancy the plan leaves out
        const listed = schema.array(schema.string()).safeParse(row?.value);
        const isUnlisted =
            typeof use !== "string" &&
            "tenancy" in use &&
            (!listed.success || !listed.data.includes(use.tenancy));
        if (row === undefined || isUnlisted) {
            return {
                status: "false",
                reason: "Unentitled",
                message: `no plan of the account grants ${used}`,
            };
        }
        // refuse a use blocked at its limit
        else if (row.state === "blocked") {
            const limit = row.limit === null ? "its spending limit" : `its limit of ${row.limit}`;

            return { status: "false", reason: "Blocked", message: `${used} reached ${limit}` };
        }
        // allow any other use
        else {
            return { status: "true", reason: "Within", message: `${used} within the plan` };
        }
    },

    /** Decide the condition of one use by an account's spaces from its replicated entitlements. */
    async check(
        database: DatabaseConnection,
        accountId: Identifier<"account">,
        use: EntitlementUse,
    ): Promise<Condition> {
        return Entitlement.decide(await Entitlement.replicated(database, accountId), use);
    },

    /** Describe a use as people read it. */
    describe(use: EntitlementUse): string {
        // describe a resource by its name
        if (typeof use === "string") {
            return use;
        }
        // describe a tenancy
        else if ("tenancy" in use) {
            return `${use.tenancy} tenancy`;
        }
        // describe a feature
        else {
            return `feature ${use.feature.name}`;
        }
    },

    /** Follow the entitlements of the accounts of the spaces a database serves. */
    follow: space.follow(entitlementShape, {
        key: async (row, view) => {
            // follow nothing for a space this database does not serve
            if (!(await Space.isServing(view, row.id))) {
                return undefined;
            }

            // follow its account's entitlements from the universe, their home
            return { key: { scope: Scope.universe.id, account: row.scope } };
        },
    }),

    /** Watch the replicated entitlements, selecting the spaces of the account they limit. */
    watches(): readonly ObjectWatch[] {
        return [
            ObjectWatch.of(entitlement.table, (row, database) => Space.within(database, row.scope)),
        ];
    },

    /** Decide a served space's database, storage, compute and network conditions. */
    async enforce(
        row: Space,
        database: DatabaseConnection,
        now: number,
    ): Promise<Readonly<Record<SpaceResource, Condition>>> {
        // decide each resource's condition from one read of the account's entitlements
        const rows = await Entitlement.replicated(database, account.identifier(row.scope));
        const conditions = {
            database: Entitlement.decide(rows, "database"),
            storage: Entitlement.decide(rows, "storage"),
            compute: Entitlement.decide(rows, "compute"),
            network: Entitlement.decide(rows, "network"),
        };

        // refuse writes within the space while its database is exceeded
        const [link] = await Scope.chain(Snapshot.live(database), row.id);
        const isRefused = conditions.database.status === "false";
        if (present(link, `the scope of space ${row.id}`).isCapped !== isRefused) {
            await Transaction.run(database, (transaction) =>
                Scope.cap(transaction, row.id, isRefused ? now : null),
            );
        }

        return conditions;
    },

    /** Refuse a domain claim in an account or organisation whose payer's plan allows no domains. */
    async requireCustomDomains(context: MethodContext): Promise<void> {
        // refuse a payer whose plan has no domain entitlement
        const row = await Entitlement.read(
            context,
            await Account.payer(context, context.scope),
            "domain",
        );
        if (row === undefined) {
            throw AccountError.of(
                "CUSTOM_DOMAINS_UNENTITLED",
                `the plan of ${context.scope} does not allow custom domains`,
            );
        }
    },
};
