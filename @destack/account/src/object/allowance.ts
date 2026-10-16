import { through, union } from "@destack/access";
import { type DatabaseConnection, type Select, Snapshot, Transaction } from "@destack/db";
import {
    type MethodContext,
    type Condition,
    defineObject,
    defineShape,
    field,
    Follow,
    ObjectWatch,
    Shape,
} from "@destack/object";
import { defineSchema, type Identifier, present, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { Replica, Scope } from "@destack/sync";
import { Account, account } from "./account.ts";
import { residency } from "./residency.ts";
import { space, Space, type SpaceResource } from "./space.ts";

/** The resources an allowance limits. */
export const ALLOWANCE_RESOURCES = [
    "database",
    "storage",
    "compute",
    "network",
    "tenancy",
    "domain",
] as const;

/** A resource an allowance limits. */
export type AllowanceResource = (typeof ALLOWANCE_RESOURCES)[number];

/** The resources whose exceeded allowance throttles a space's requests. */
export const THROTTLED_RESOURCES = [
    "compute",
    "network",
] as const satisfies readonly SpaceResource[];

/** Where a space's work runs: shared or dedicated cloud machines, or its people's own. */
export const Tenancy = defineSchema(schema.enum(["shared", "dedicated", "own"]));
/** Where a space's work runs. */
export type Tenancy = schema.Infer<typeof Tenancy>;

/** Where a metered usage stands against its limit and its subscription's budget. */
export const USAGE_STATES = ["within", "near", "over", "blocked"] as const;

/** Where a metered usage stands. */
export type UsageState = (typeof USAGE_STATES)[number];

/** What a plan grants on a resource. */
export const AllowanceGrant = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** A metered usage. */
            kind: schema.literal("metered"),
            /** The most usage, or null for usage bounded by the spending limit alone. */
            limit: schema.number().nullable(),
            /** The usage so far, in the limit's unit. */
            used: schema.number(),
            /** Where the usage stands against the limit. */
            state: schema.enum(USAGE_STATES),
        }),
        schema.object({
            /** A count its enforcer keeps, such as members. */
            kind: schema.literal("counted"),
            /** The largest count, or null for unlimited. */
            limit: schema.number().nullable(),
        }),
        schema.object({
            /** The values the plan lists, such as the tenancies spaces run in. */
            kind: schema.literal("listed"),
            /** The listed values. */
            values: schema.array(schema.string().min(1)),
        }),
        schema.object({
            /** Access to the resource alone. */
            kind: schema.literal("access"),
        }),
    ]),
);
/** What a plan grants on a resource. */
export type AllowanceGrant = schema.Infer<typeof AllowanceGrant>;

/** The name of the shape replicating one account's allowances. */
const ALLOWANCE_SHAPE = "allowance";

/** The name of the shape replicating every account's allowances a publisher keeps. */
const UNIVERSE_ALLOWANCE_SHAPE = "allowance:universe";

/** An account's allowance on a resource. */
export const allowance = defineObject(
    {
        name: "allowance",
        plural: "allowances",
        scope: space,
        owner: "account",
        fields: {
            /** The account the allowance limits. */
            account: field.reference(account).relate("account"),
            /** The resource allowed. */
            resource: field.enum(ALLOWANCE_RESOURCES),
            /** What the plan grants on it. */
            grant: field.json(AllowanceGrant),
        },
        permissions: {
            // let the account's readers and its residency's servers read
            read: union(through("account", "read"), through("account", "serve")),
            // let its residency's servers keep it from the plan
            keep: through("account", "serve"),
        },
        reserved: ["keep"],
        constraints: {
            resource: { kind: "index", on: ["accountId", "resource"], unique: true },
        },
    },
    (method) => ({
        /** Keep the allowance a plan grants on a resource. */
        create: method.create("keep", { isInternal: true }),
        /** Change an allowance as the plan or its usage changes. */
        update: method.update("keep", { isInternal: true }),
        /** Remove an allowance no plan grants any longer. */
        delete: method.delete("keep", { isInternal: true }),
    }),
);
/** An account's allowance on a resource, as its table stores it. */
export type Allowance = Select<typeof allowance.table>;

/** The parameters of a replica of one account's allowances. */
const AllowanceParameters = schema.object({
    /** The account whose allowances are replicated. */
    account: schema.identifier("account"),
});

/** The shape replicating one account's allowances from the home space keeping them. */
export const allowanceShape = defineShape({
    name: ALLOWANCE_SHAPE,
    parameters: AllowanceParameters,
    objects: [allowance],
    queries: ({ account: accountId }) => [allowance.query({ where: { account: accountId } })],
});

/** The shape replicating every account's allowances a residency's home keeps. */
export const universeAllowanceShape = defineShape({
    name: UNIVERSE_ALLOWANCE_SHAPE,
    parameters: schema.object({}),
    objects: [allowance],
    queries: () => [allowance.query()],
});

/** The condition of a use before an account's allowances finish replicating. */
const PENDING: Condition = {
    status: "true",
    reason: "Pending",
    message: "waiting for the account's plan",
};

/** A use a machine checks against an account's allowances. */
export type AllowanceUse = SpaceResource | { readonly tenancy: Tenancy };

/** Account allowances as their followers replicate and enforce them. */
export const Allowance = {
    /** Follow every account's allowances from each residency's home, as the universe does. */
    universe: Follow.of(universeAllowanceShape, {
        rows: () => residency.query(),
        key: async (row) =>
            typeof row["home"] === "string" ? { key: { scope: row["home"] } } : undefined,
        // serve while the allowances arrive, deciding uses pending until they do
        isReadinessGate: false,
    }),

    /** Read an account's replicated allowance on a resource, absent where no plan grants it. */
    async read(
        reader: Pick<Snapshot, "read">,
        accountId: Identifier<"account">,
        resource: AllowanceResource,
    ): Promise<Allowance | undefined> {
        return reader.read(allowance.table).findFirst({ where: { accountId, resource } });
    },

    /** Decide the condition of a use by an account's spaces from its replicated allowances. */
    async check(
        database: DatabaseConnection,
        accountId: Identifier<"account">,
        use: AllowanceUse,
    ): Promise<Condition> {
        // wait for the replica's first complete snapshot of the account's allowances, wherever it is held
        const [replicated] = await Shape.replicas(database, allowanceShape, { account: accountId });
        if (
            replicated === undefined ||
            !(await Replica.isSynced(database, replicated.name, replicated.scope))
        ) {
            return PENDING;
        }

        // read the account's allowance on the resource
        const resource = typeof use === "string" ? use : "tenancy";
        const row = await Allowance.read(Snapshot.live(database), accountId, resource);
        const used = typeof use === "string" ? use : `${use.tenancy} tenancy`;

        // refuse a use no plan grants
        const grant = row?.grant;
        if (
            grant === undefined ||
            (typeof use !== "string" &&
                (grant.kind !== "listed" || !grant.values.includes(use.tenancy)))
        ) {
            return {
                status: "false",
                reason: "Unentitled",
                message: `no plan of the account grants ${used}`,
            };
        }
        // refuse a use blocked at its limit
        else if (grant.kind === "metered" && grant.state === "blocked") {
            const limit = grant.limit === null ? "its spending limit" : `${grant.limit}`;

            return {
                status: "false",
                reason: "Blocked",
                message: `${used} used ${grant.used} of ${limit}`,
            };
        }
        // allow any other use
        else {
            return { status: "true", reason: "Within", message: `${used} within the plan` };
        }
    },

    /** Refuse a domain claim in an account or organisation whose payer's plan allows no domains. */
    async requireCustomDomains(context: MethodContext): Promise<void> {
        // refuse a payer whose plan has no domain allowance
        const row = await Allowance.read(
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

/** A machine's enforcement of account allowances on its spaces. */
export const SpaceAllowance = {
    /** Read a space's allowance conditions, and refuse an unknown space. */
    async read(
        snapshot: Pick<Snapshot, "read">,
        spaceId: string,
    ): Promise<Pick<Space, "id" | "conditions">> {
        const id = schema.identifier("space").parse(spaceId);

        return present(
            await snapshot.read(space.table).findFirst({ where: { id } }),
            `space ${spaceId}`,
        );
    },

    /** Report whether a space's allowance on a resource is exceeded. */
    isExceeded(row: Pick<Space, "conditions">, resource: SpaceResource): boolean {
        return row.conditions[resource]?.status === "false";
    },

    /** Find the first exceeded resource that throttles a space's requests, absent when none is. */
    throttled(row: Pick<Space, "conditions">): SpaceResource | undefined {
        return THROTTLED_RESOURCES.find((resource) => SpaceAllowance.isExceeded(row, resource));
    },

    /** Follow the allowances of the accounts of the spaces a database serves. */
    follow: Follow.of(allowanceShape, {
        rows: () => space.query(),
        key: async (row, view) => {
            // follow nothing for a space this database does not serve
            const spaceId = space.identifier(schema.string().parse(row["id"]));
            if (!(await Space.isServing(view.database, spaceId))) {
                return undefined;
            }

            // follow its account's allowances from the home of the account's residency
            const accountId = account.identifier(schema.string().parse(row["scope"]));
            const owner = present(
                await view.read(account.table).findFirst({ where: { id: accountId } }),
                `the account ${accountId}`,
            );
            const residing = present(
                await view.read(residency.table).findFirst({ where: { id: owner.residencyId } }),
                `the residency ${owner.residencyId}`,
            );

            return {
                key: {
                    scope: present(residing.home, `the home of the residency ${residing.id}`),
                    account: accountId,
                },
            };
        },
    }),

    /** Watch the replicated allowances, selecting the spaces of the account they limit. */
    watches(): readonly ObjectWatch[] {
        return [
            ObjectWatch.of(allowance.table, (row, database) =>
                Space.within(database, row.accountId),
            ),
        ];
    },

    /** Refuse a resource use beyond a space's allowance, with the condition's message. */
    require(row: Pick<Space, "id" | "conditions">, resource: SpaceResource): void {
        // refuse an exceeded resource with its condition's message
        if (SpaceAllowance.isExceeded(row, resource)) {
            const { message } = present(row.conditions[resource], `the ${resource} condition`);
            throw new CallError("QUOTA_EXCEEDED", {
                message: `space ${row.id} exceeds its allowance: ${message}`,
            });
        }
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
            database: await Allowance.check(database, accountId, "database"),
            storage: await Allowance.check(database, accountId, "storage"),
            compute: await Allowance.check(database, accountId, "compute"),
            network: await Allowance.check(database, accountId, "network"),
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
};
