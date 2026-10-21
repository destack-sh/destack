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
import { CallError } from "@destack/object";
import { Replica, Scope } from "@destack/sync";
import { Account, account } from "./account.ts";
import { space, Space, type SpaceResource } from "./space.ts";

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

/** Where a space's work runs: shared or dedicated cloud machines, or its people's own. */
export const Tenancy = schema.enum(["shared", "dedicated", "own"]);
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
            /** The platform resource the feature allows, absent for a package's own feature. */
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

/** The condition of a use before an account's entitlements finish replicating. */
const PENDING: Condition = {
    status: "true",
    reason: "Pending",
    message: "waiting for the account's plan",
};

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
        granted: EntitlementResource | { readonly feature: FeatureReference },
    ): Promise<Entitlement | undefined> {
        const where =
            typeof granted === "string"
                ? { scope: accountId, resource: granted }
                : {
                      scope: accountId,
                      packageId: granted.feature.packageId,
                      feature: granted.feature.name,
                  };

        return reader.read(entitlement.table).findFirst({ where });
    },

    /** Decide the condition of a use by an account's spaces from its replicated entitlements. */
    async check(
        database: DatabaseConnection,
        accountId: Identifier<"account">,
        use: EntitlementUse,
    ): Promise<Condition> {
        // wait for the replica's first complete snapshot of the account's entitlements, wherever it lives
        const [replicated] = await entitlementShape.replicas(database, {
            account: accountId,
        });
        if (
            replicated === undefined ||
            !(await Replica.isSynced(database, replicated.name, replicated.scope))
        ) {
            return PENDING;
        }

        // read the account's entitlement to the resource or feature
        const granted = typeof use === "string" ? use : "tenancy" in use ? "tenancy" : use;
        const row = await Entitlement.read(Snapshot.live(database), accountId, granted);
        const used =
            typeof use === "string"
                ? use
                : "tenancy" in use
                  ? `${use.tenancy} tenancy`
                  : `feature ${use.feature.name}`;

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
        // decide each resource's condition
        const accountId = account.identifier(row.scope);
        const conditions = {
            database: await Entitlement.check(database, accountId, "database"),
            storage: await Entitlement.check(database, accountId, "storage"),
            compute: await Entitlement.check(database, accountId, "compute"),
            network: await Entitlement.check(database, accountId, "network"),
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
            throw new CallError("QUOTA_EXCEEDED", {
                message: `the plan of ${context.scope} allows no custom domains`,
            });
        }
    },
};
