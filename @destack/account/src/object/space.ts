import {
    eq,
    sql,
    type DatabaseConnection,
    type Select,
    type SQL,
    type SQLWrapper,
    type Snapshot,
} from "@destack/db";
import { placement } from "./placement.ts";
import { defineObject, field, type KeyFields, ObjectWatch } from "@destack/object";
import { RISKS } from "@destack/resource";
import { type Identifier, present, schema } from "@destack/schema";
import { replica, Scope } from "@destack/sync";
import { account } from "./account.ts";
import { organization } from "./organization.ts";
import { StackSource } from "./stack-source.ts";
import { sudo } from "../authentication/sudo.ts";
import {
    anyone,
    contained,
    type Creation,
    none,
    permission,
    principal,
    relation,
    through,
    union,
} from "@destack/access";
import { SPACE_NAME_LENGTH, SpaceName } from "../address/address.ts";
import { Label } from "../address/label.ts";
import { AccountError } from "../error/error.ts";

/** The longest space title, in characters: a line of a sidebar with room to spare. */
const TITLE_LENGTH = 128;

/** An isolated place for installations and their data. */
export const space = defineObject(
    {
        name: "space",
        plural: "spaces",
        scope: account,
        represents: principal.space,
        isScope: true,
        controlled: true,
        fields: {
            /** The unique address within the account. */
            name: field.string(SpaceName),
            /** The display text, absent to show the name. */
            title: field.string(schema.string().min(1).max(TITLE_LENGTH)).optional(),
            /** The least risk a resource plan waits for approval at. */
            approval: field.enum(RISKS).default("backward-incompatible"),
            /** The stack applied to the space. */
            stack: field.json(StackSource),
        },
        indexes: { name: { on: ["name"], across: () => account } },
        relations: {
            // the outsiders who find the space without reading it
            guest: { subjects: [principal.user, anyone.all()], grantedBy: "manage" },
            // the machine the space moves to, which its placement relates while the move runs
            receiver: { subjects: [principal.machine], grantedBy: "receive" },
        },
        permissions: {
            // let the machines serving the account and each space itself read the space's row, which their replicas follow
            read: union(permission("serve"), contained(principal.space)),
            // let the space's readers, its guests and the installations living in it find it
            list: union(permission("read"), relation("guest"), contained()),
            create: permission("write"),
            update: permission("manage"),
            delete: permission("manage"),
            rotate: permission("manage"),
            // let the platform move a space it declares to another stack
            upgrade: none(),
            // leave the space's resources to its own stack, which its base installations declare
            provision: none(),
            // let the platform relate the machine a move targets
            receive: none(),
            // let the space and its installations, the machine it moves to and the account's servers replicate its chain
            replicate: union(contained(), relation("receiver"), through("account", "serve")),

            // let the space's readers read its operational telemetry and product analytics, its managers unmasked
            observe: permission("read"),
            analyze: permission("read"),
            unmask: permission("manage"),
            // let the machines serving the account forward the space's telemetry
            serve: through("account", "serve"),
        },
        reserved: ["replicate", "upgrade", "receive", "serve", "provision"],
        shareable: { isPublic: true },
        suspendable: { by: "update" },
        elevated: { delete: sudo, rotate: sudo },
        administration: ["read", "list", "update", "manage"],
        constraints: {
            scopeId: { kind: "unique", on: ["scope", "id"] },
            name: { kind: "check", condition: { name: Label.condition(SPACE_NAME_LENGTH) } },
        },
    },
    (method) => ({
        list: method.list("list"),
        // make a new space the owner of itself, since its code acts in it
        create: method.create("create", {
            fields: ["name", "title", "stack"],
            filled: ["stack"],
            input: schema.object({
                /** The machine to place it on. */
                machine: schema.identifier("machine").exactOptional(),
            }),
        }),
        update: method.update("update", { fields: ["title", "approval"] }),

        /** Move the space to another stack, which its machine applies. */
        upgrade: method.update("upgrade", { fields: ["stack"], isInternal: true }),

        /** Move a space to another address within its account. */
        rename: method.mutation({
            permission: "update",
            input: schema.object({
                /** The new address. */
                name: SpaceName,
            }),
        }),

        delete: method.delete("delete"),

        /** Read the space's address `<name>--<handle>` in its account. */
        address: method.query({
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

/** The resources whose use sets a space's conditions. */
export type SpaceResource = "database" | "storage" | "compute" | "network";

/** The resources whose exceeded entitlement throttles a space's requests. */
export const THROTTLED_RESOURCES = [
    "compute",
    "network",
] as const satisfies readonly SpaceResource[];

/** The space checks, reads and replicates of a serving machine. */
export const Space = {
    /** Build the access a space's creation writes: the space is the owner of itself. */
    creation(row: Pick<Space, "id">): Creation {
        return { owner: principal.space.reference(Scope.universe.id, row.id) };
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
                "ENTITLEMENT_EXCEEDED",
                `space ${row.id} exceeds its entitlement: ${message}`,
            );
        }
    },

    /** Watch the directory's placements, and select the keys of a space whose placement changed. */
    watch(
        keys: (
            spaceId: Identifier<"space">,
            database: DatabaseConnection,
        ) => PromiseLike<readonly KeyFields[]>,
    ): ObjectWatch<typeof placement.table> {
        return ObjectWatch.of(placement.table, async (row, database) =>
            keys(schema.identifier("space").parse(row.id), database),
        );
    },
};
