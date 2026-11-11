import { none, through, union } from "@destack/access";
import { type DatabaseConnection, type Select, Snapshot, Transaction } from "@destack/db";
import {
    type MethodContext,
    type Condition,
    defineObject,
    defineShape,
    field,
    ObjectWatch,
} from "@destack/object";
import { type Package, PackageId } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import type { FeatureReference } from "@destack/object";
import { canonicalize, type Identifier, present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { Account, account } from "./account.ts";
import { space, Space, type SpaceResource } from "./space.ts";
import { Tenancy } from "./space.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";
import {
    ENTITLEMENT_RESOURCES,
    type Allowance,
    type EntitlementResource,
    FEATURE_KINDS,
    USAGE_STATES,
} from "./allowance.ts";

/** A retention setting's definition, read for the days it keeps events by default. */
const RetentionSetting = schema.looseObject({ default: schema.number().int().positive() });

/** Read the days a scope retains a kind's events by its setting, as event stores read them. */
export type RetentionReader = (setting: object, owner: Package, scope: string) => Promise<number>;

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
            // let the account's home set it alone, as account.entitle does
            provide: none(),
        },
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
    // let the account's home write them as its entitlements change, through account.entitle
    (method) => ({
        create: method.create("provide", { visibility: "package" }),
        update: method.update("provide", {
            visibility: "package",
            fields: ["kind", "value", "limit", "resetAt", "state", "resource"],
        }),
        delete: method.delete("provide", { visibility: "package" }),
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

/** A use a machine checks against an account's entitlements. */
export type EntitlementUse =
    | SpaceResource
    | { readonly tenancy: Tenancy }
    | { readonly feature: FeatureReference };

/** Account entitlements as their followers replicate and enforce them. */
export const Entitlement = {
    /** Key an entitlement or allowance by its feature. */
    key(allowance: Pick<Allowance, "packageId" | "feature">): string {
        return DeclarationReference.key({
            packageId: allowance.packageId,
            name: allowance.feature,
        });
    },

    /** Read what an entitlement allows as an allowance, without its account and row. */
    allowance(row: Entitlement): Allowance {
        const { packageId, feature, kind, value, limit, resetAt, state, resource } = row;

        return { packageId, feature, kind, value, limit, resetAt, state, resource };
    },

    /** Check whether an entitlement allows what an allowance of its feature does. */
    isSame(row: Entitlement, wanted: Allowance): boolean {
        return (
            row.kind === wanted.kind &&
            canonicalize(row.value) === canonicalize(wanted.value) &&
            row.limit === wanted.limit &&
            row.resetAt === wanted.resetAt &&
            row.state === wanted.state &&
            row.resource === wanted.resource
        );
    },

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
        entry: Pick<Allowance, "resource" | "packageId" | "feature">,
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
        // wait for the replica's first complete snapshot of the account's entitlements from the universe
        const key = { scope: Scope.universe.id, account: accountId };
        if (!(await entitlementShape.isSynced(database, key))) {
            return undefined;
        }

        return Snapshot.live(database)
            .read(entitlement.table)
            .findMany({ where: { scope: accountId } });
    },

    /** Cap a space's event retention at each setting's default unless its account keeps more. */
    capRetention(database: DatabaseConnection, retention: RetentionReader): RetentionReader {
        return async (setting, owner, scope) => {
            // read the days the space chose and the entitlements of its account, which the follow replicates
            const days = await retention(setting, owner, scope);
            const { id } = await Account.of(Snapshot.live(database), scope);
            const rows = await Entitlement.replicated(database, id);
            if (rows === undefined) {
                throw AccountError.of(
                    "ENTITLEMENTS_UNAVAILABLE",
                    `the entitlement list of account '${id}' is unavailable: it has not replicated yet`,
                );
            }

            return Entitlement.retention(rows, days, setting);
        };
    },

    /** Keep a space's chosen days, capped at its setting's default without custom retention. */
    retention(rows: readonly Allowance[], days: number, setting: object): number {
        const isCustom = rows.some((row) => Entitlement.matches(row, "retention"));

        return isCustom ? days : Math.min(days, RetentionSetting.parse(setting).default);
    },

    /** Decide the condition of a use from an account's replicated entitlements. */
    decide(rows: readonly Allowance[] | undefined, use: EntitlementUse): Condition {
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

            return { status: "false", reason: "Blocked", message: `${used} is at ${limit}` };
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
    follow: entitlementShape.per(space, {
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

    /** Decide a served space's database, bucket, compute and network conditions. */
    async enforce(
        row: Space,
        database: DatabaseConnection,
        now: number,
    ): Promise<Readonly<Record<SpaceResource, Condition>>> {
        // decide each resource's condition from one read of the account's entitlements
        const rows = await Entitlement.replicated(database, account.identifier(row.scope));
        const conditions = {
            database: Entitlement.decide(rows, "database"),
            bucket: Entitlement.decide(rows, "bucket"),
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

    /** Refuse a domain claim in an account or organization whose payer's plan allows no domains. */
    async requireCustomDomains(context: MethodContext): Promise<void> {
        // refuse a payer whose plan has no domain entitlement
        const row = await Entitlement.read(
            context,
            await Account.payer(context, context.scope),
            "domain",
        );
        if (row === undefined) {
            throw AccountError.of(
                "CUSTOM_DOMAIN_EXHAUSTED",
                `a custom domain of '${context.scope}' exceeds its plan: the plan does not allow custom domains`,
            );
        }
    },
};
