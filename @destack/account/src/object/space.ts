import { eq, sql, type Select, type SQL, type SQLWrapper } from "@destack/db";
import type { DatabaseConnection, Snapshot, Watch } from "@destack/db/database";
import {
    approval,
    defineObject,
    field,
    type KeyFields,
    controlled,
    shareable,
    StackTarget,
    suspendable,
} from "@destack/object";
import { RISKS } from "@destack/resource";
import { type Identifier, present, schema } from "@destack/schema";
import { Scope, replica } from "@destack/sync";
import { Account, account } from "./account.ts";
import { organization } from "./organization.ts";
import { StackRelease } from "./stack-release.ts";
import { sudo } from "../authentication/sudo.ts";
import {
    anyone,
    contained,
    none,
    permission,
    principal,
    relation,
    union,
    type Creation,
} from "@destack/access";
import { SPACE_NAME_LENGTH, SpaceName } from "../address/address.ts";
import { Label } from "../address/label.ts";
import { AccountError } from "../error/error.ts";
import { machine } from "./machine.ts";
import { argument, plural, t } from "@destack/locale";

/** The tenancies: another account's machines, or the account's managed or self-hosted ones. */
export const TENANCIES = ["shared", "dedicated", "self"] as const;

/** Where a space's work runs: another account's machines, or the account's own. */
export const Tenancy = schema.enum(TENANCIES);
/** Where a space's work runs. */
export type Tenancy = schema.Infer<typeof Tenancy>;

/** The longest space title, in characters: a line of a sidebar with room to spare. */
const TITLE_LENGTH = 128;

/** An isolated place for installations and their data. */
export const space = defineObject(
    {
        name: "space",
        plural: "spaces",
        title: t`${plural(argument("count"), { one: "Space", other: "Spaces" })}`,
        scope: account,
        fields: {
            /** The unique address within the account. */
            name: field.string(SpaceName),
            /** The display text, absent to show the name. */
            title: field.string(schema.string().min(1).max(TITLE_LENGTH)).optional(),
            /** The least risk a resource plan waits for approval at. */
            approval: field.enum(RISKS).default("backward-incompatible"),
            /** The stack applied to the space. */
            stack: field.json(StackRelease),
            /** Where a person chose the space's work runs, absent to keep it on its machine. */
            tenancy: field.enum(TENANCIES).optional(),
        },
        indexes: { name: { on: ["name"], across: () => account, renamed: "former" } },
        presentable: { card: ["name", "tenancy"] },
        // keep each change, as the tracked objects read through a space decide their past reads by it
        retention: "history",
        constraints: {
            scopeId: { kind: "unique", on: ["scope", "id"] },
            name: { kind: "check", condition: { name: Label.condition(SPACE_NAME_LENGTH) } },
        },
        container: "scope",
        access: {
            represents: principal.space,
            // let a change ask the space's managers as one set, as its plans do
            contributes: [{ object: () => approval, relation: "approver", members: "manage" }],
            relations: {
                // the outsiders who find the space without reading it
                guest: { subjects: [principal.user, anyone.all()], grantedBy: "manage" },
                // the machine the space moves to, which its placement relates while the move runs
                receiver: { subjects: [principal.machine], grantedBy: "receive" },
                // the machine the space is placed on, related by its placement
                machine: { subjects: [principal.machine], grantedBy: "receive" },
                // the home of the space's account, related as the space is created
                home: { subjects: [principal.space], grantedBy: "receive" },
            },
            permissions: {
                // let the machines serving the space and each space itself read the space's row, which their replicas follow
                read: union(permission("serve"), contained(principal.space)),
                // let the space's readers, its guests and the installations living in it find it
                list: union(permission("read"), relation("guest"), contained()),
                create: permission("write"),
                update: permission("manage"),
                delete: permission("manage"),
                rotate: permission("manage"),
                // let the platform move a space it declares to another stack
                upgrade: none(),
                // leave the space's resources to its own stack, which declares them
                provision: none(),
                // let the platform relate the machine a move targets
                receive: none(),
                // let the space and its installations and the machines serving it or receiving it replicate its chain
                replicate: union(contained(), relation("receiver"), permission("serve")),

                // let the space's readers read its operational telemetry and product analytics, its managers unmasked
                observe: permission("read"),
                analyze: permission("read"),
                unmask: permission("manage"),
                // let the machine it is placed on and its account's home serve it
                serve: union(relation("machine"), relation("home")),
            },
            reserved: ["replicate", "upgrade", "receive", "serve", "provision"],
            elevated: { delete: sudo, rotate: sudo },
            suspended: ["read", "list", "update", "manage"],
        },
        traits: [controlled(), shareable({ isPublic: true }), suspendable({ by: "update" })],
    },
    (method) => ({
        list: method.list("list"),
        // make a new space the owner of itself, since its code acts in it
        create: method.create("create", {
            fields: ["name", "title", "stack"],
            filled: ["stack"],
            input: schema.object({
                /** The machine to place it on, by its identifier or its name. */
                machine: StackTarget.on(
                    schema.lazy(() => machine.addressSchema),
                    () => machine,
                ).exactOptional(),
            }),
        }),
        update: method.update("update", { fields: ["title", "approval", "tenancy"] }),

        /** Move the space to another stack, which its machine applies. */
        upgrade: method.update("upgrade", { fields: ["stack"], visibility: "package" }),

        /** Move a space to another address within its account. */
        rename: method.mutation({
            title: t`Rename`,
            permission: "update",
            input: schema.object({
                /** The new address. */
                name: SpaceName,
            }),
        }),

        delete: method.delete("delete"),

        /** Read the space's address `<name>--<handle>` in its account. */
        address: method.query({
            title: t`Read address`,
            permission: "read",
            output: schema.object({
                /** The address. */
                address: schema.string().min(1),
            }),
        }),
    }),
).implement({ methods: { rename: (call) => call.update({ name: call.input.name }) } });
/** A space row in the space's database. */
export type Space = Select<typeof space.table>;

/** The resources whose use sets a space's conditions, each a condition of its name. */
export const SPACE_RESOURCES = ["database", "bucket", "compute", "network"] as const;

/** A resource whose use sets a space's condition. */
export type SpaceResource = (typeof SPACE_RESOURCES)[number];

/** The resources whose exceeded entitlement throttles a space's requests. */
export const THROTTLED_RESOURCES = [
    "compute",
    "network",
] as const satisfies readonly SpaceResource[];

/** The space checks, reads and replicates of a serving machine. */
export const Space = {
    /** Build a space's creation access: itself as owner, its home as server, a home's guests. */
    creation(row: Pick<Space, "id" | "scope">): Creation {
        // relate the account's home, which serves its spaces
        const home = Account.home(account.identifier(row.scope));
        const served = {
            relation: "home",
            subject: principal.space.reference(Scope.universe.id, home),
        };

        // let anyone find an account's home, whose objects each decide who reads them
        const open =
            row.id === home ? [{ relation: "guest", subject: anyone.reference("*", "*") }] : [];

        return {
            owner: principal.space.reference(Scope.universe.id, row.id),
            relationships: [served, ...open],
        };
    },

    /** List the spaces of the accounts a payer pays for: its accounts and its organizations'. */
    async paidBy(snapshot: Snapshot, payer: Identifier<"account">): Promise<Identifier<"space">[]> {
        // take the payer itself, and the accounts of the organizations it pays for
        const paid = await snapshot.read(organization.table).findMany({ where: { payer } });
        const accounts =
            paid.length === 0
                ? []
                : await snapshot
                      .read(account.table)
                      .findMany({ where: { scope: { in: paid.map((row) => row.id) } } });
        const accountIds = [payer, ...accounts.map((row) => row.id)];

        // list their spaces
        const rows = await snapshot
            .read(space.table)
            .findMany({ where: { scope: { in: accountIds } } });

        return rows.map((row) => row.id);
    },

    /** The rows `isServing` reads beside the space's own: its scope and its replicas. */
    serving: [
        { table: Scope.table, fields: ["fencedAt"], on: { id: "scope" } },
        { table: replica, fields: [], on: { id: "scope" } },
    ] satisfies Watch[],

    /** Report whether a space's database serves the space now. */
    async isServing(view: Pick<Snapshot, "read">, spaceId: Identifier<"space">): Promise<boolean> {
        // read the space, its scope and a replica of it
        const [row, scope, existing] = await Promise.all([
            view.read(space.table).findFirst({ where: { id: spaceId } }),
            view.read(Scope.table).findFirst({ where: { scope: spaceId } }),
            view.read(replica).findFirst({ where: { scope: spaceId } }),
        ]);

        return row !== undefined && scope?.fencedAt === null && existing === undefined;
    },

    /** Select the keys of the spaces within a space or account scope. */
    async within(database: DatabaseConnection, scope: string): Promise<KeyFields[]> {
        // take a space as itself
        const spaceId = schema.identifier("space").safeParse(scope);
        if (spaceId.success) {
            return [{ id: spaceId.data }];
        }

        // select an account's spaces
        const accountId = schema.identifier("account").safeParse(scope);

        return accountId.success
            ? database
                  .select({ id: space.table.id })
                  .from(space.table)
                  .where(eq(space.table.scope, accountId.data))
            : [];
    },

    /** Match the space a database serves by its identifier: born here, unfenced and no replica. */
    served(id: SQLWrapper = space.table.id): SQL {
        return sql`EXISTS (SELECT 1 FROM ${Scope.table} WHERE ${Scope.table.scope} = ${id} AND ${Scope.table.fencedAt} IS NULL) AND NOT EXISTS (SELECT 1 FROM ${replica} WHERE ${replica.scope} = ${id})`;
    },

    /** Match a space a database has not given birth to by its identifier: no scope, no replica. */
    unborn(id: SQLWrapper = space.table.id): SQL {
        return sql`NOT EXISTS (SELECT 1 FROM ${Scope.table} WHERE ${Scope.table.scope} = ${id}) AND NOT EXISTS (SELECT 1 FROM ${replica} WHERE ${replica.scope} = ${id})`;
    },

    /** Read a space's row, refusing an unknown space. */
    async read(snapshot: Pick<Snapshot, "read">, spaceId: string): Promise<Space> {
        const id = schema.identifier("space").parse(spaceId);

        return present(
            await snapshot.read(space.table).findFirst({ where: { id } }),
            `space ${spaceId}`,
        );
    },

    /** Report whether a space's entitlement to a resource is exceeded. */
    isExceeded(row: Pick<Space, "conditions">, resource: SpaceResource): boolean {
        return row.conditions[resource]?.status === "false";
    },

    /** Find the first exceeded resource that throttles a space's requests, absent when none is. */
    throttled(row: Pick<Space, "conditions">): SpaceResource | undefined {
        return THROTTLED_RESOURCES.find((resource) => Space.isExceeded(row, resource));
    },

    /** Refuse a resource use beyond a space's entitlement, with the condition's message. */
    requireWithin(row: Pick<Space, "id" | "conditions">, resource: SpaceResource): void {
        // refuse an exceeded resource with its condition's message
        if (Space.isExceeded(row, resource)) {
            const { message } = present(row.conditions[resource], `the ${resource} condition`);
            throw AccountError.of(
                "ENTITLEMENT_EXHAUSTED",
                `space '${row.id}' exceeds its entitlement: ${message}`,
            );
        }
    },
};
