import {
    account,
    Entitlement,
    type Tenancy,
    machine,
    ResidencyCode,
    Space,
    space,
} from "@destack/account/object";
import { and, desc, type DatabaseConnection, eq, isNull } from "@destack/db";
import { Placement } from "@destack/account/object";
import { ObjectWatch, type Condition } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { type Identifier, present } from "@destack/schema";
import { setting, SettingValue } from "@destack/setting/object";
import { transfer } from "../../object/index.ts";
import { tenancy } from "../../setting/index.ts";
import { RequestId } from "@destack/runtime/request";
import type { SpaceAccounts } from "../server.ts";

/** What a machine schedules its spaces with. */
export interface SchedulerOptions {
    /** The machine serving the spaces. */
    readonly machine: Identifier<"machine">;
    /** The account package, which lists the cloud machines serving each residency. */
    readonly accounts: Pick<SpaceAccounts, "residency">;
    /** Whether the machine decides its spaces' tenancies by their accounts' entitlements. */
    readonly isEnforcingEntitlements: boolean;
}

/** Where a tenancy runs a space: the machine to move it to, or the condition refusing all. */
type Target =
    | { readonly kind: "machine"; readonly machine: Identifier<"machine"> }
    | { readonly kind: "refused"; readonly condition: Condition };

/** The scheduler of served spaces onto the machines their tenancies ask for. */
export class Scheduler {
    /** What the machine schedules its spaces with. */
    readonly #options: SchedulerOptions;

    /** Schedule a machine's spaces through the residencies' cloud machines and the entitlements. */
    constructor(options: SchedulerOptions) {
        this.#options = options;
    }

    /** Watch the tenancy values, transfers and account machines for the spaces affected. */
    get watches(): readonly ObjectWatch[] {
        return [
            ObjectWatch.of(setting.table, (row, database) => {
                const { packageId, name } = tenancy.reference;
                const isTenancy = row.packageId === packageId && row.name === name;

                return isTenancy ? Space.within(database, row.scope) : [];
            }),
            ObjectWatch.of(transfer.table, (row) => [{ id: row.scope }]),
            ObjectWatch.of(machine.table, async (row, database) =>
                row.id === this.#options.machine
                    ? await database
                          .select({ id: space.table.id })
                          .from(space.table)
                          .where(Space.served())
                    : await Space.within(database, row.scope),
            ),
            Space.watch(async (spaceId) => [{ id: spaceId }]),
        ];
    }

    /** Decide where a served space belongs and start its move there. */
    async schedule(
        row: Space,
        store: Pick<ObjectStore, "database" | "serving" | "mutate">,
        now: number,
    ): Promise<Condition> {
        // wait for a running move, the account's plan or a person's choice, or keep a space where its tenancy asks
        const { database } = store;
        const requested = await Scheduler.#requested(database, row.id);
        const decision = await this.#decide(database, row, requested);
        if (decision.kind === "stay") {
            return decision.condition;
        }

        // move the space to the one machine of its tenancy
        const target = await this.#target(database, row, requested.tenancy);
        if (target.kind === "refused") {
            return target.condition;
        }
        const { machine: moving } = target;
        await store.mutate(store.serving(transfer), {
            id: RequestId.create(now),
            calls: [
                transfer.call("create", {
                    ...transfer.in(row.id),
                    id: transfer.generateId(),
                    targetId: moving,
                }),
            ],
        });

        return { status: "unknown", reason: "Moving", message: `moving to ${moving}` };
    }

    /** Decide whether a space stays where it is, and why, or moves. */
    async #decide(
        database: DatabaseConnection,
        row: Space,
        requested: { readonly tenancy: Tenancy; readonly isDefault: boolean },
    ): Promise<
        { readonly kind: "stay"; readonly condition: Condition } | { readonly kind: "move" }
    > {
        // wait for a running move
        const [moving] = await database
            .select({ target: transfer.table.targetId })
            .from(transfer.table)
            .where(and(eq(transfer.table.scope, row.id), isNull(transfer.table.completedAt)))
            .orderBy(desc(transfer.table.createdAt))
            .limit(1);
        if (moving !== undefined) {
            const message = `moving to ${moving.target}`;

            return { kind: "stay", condition: { status: "unknown", reason: "Moving", message } };
        }

        // read the allowed tenancy and this machine's tenancy
        const allowed = await this.#allowed(database, row.scope, requested.tenancy);
        const local = await this.#tenancy(database);

        // wait for this machine's own row, which its directory replica brings
        if (local === undefined) {
            const message = `waiting for the replica of machine ${this.#options.machine}`;

            return { kind: "stay", condition: { status: "unknown", reason: "Pending", message } };
        }
        // wait for the account's plan or refuse a tenancy it does not unlock
        else if (allowed.reason === "Pending" || allowed.status === "false") {
            const status = allowed.status === "false" ? "false" : "unknown";
            const { reason, message } = allowed;

            return { kind: "stay", condition: { status, reason, message } };
        }
        // keep a space on this machine when its tenancy matches
        else if (requested.tenancy === local) {
            const message = `placed ${local} on ${this.#options.machine}`;

            return { kind: "stay", condition: { status: "true", reason: "Placed", message } };
        }
        // keep a space on its machine until a person sets its tenancy
        else if (requested.isDefault && local === "own") {
            const message = `the space keeps the default tenancy ${requested.tenancy} on its machine until ${tenancy.name} is set`;

            return { kind: "stay", condition: { status: "false", reason: "Unset", message } };
        }
        // move it to its tenancy's machine
        else {
            return { kind: "move" };
        }
    }

    /** Read the tenancy of this machine's spaces, absent before its row is replicated. */
    async #tenancy(database: DatabaseConnection): Promise<Tenancy | undefined> {
        // read the kind and residency the machine's replica records
        const [row] = await database
            .select({ kind: machine.table.kind, residencyId: machine.table.residencyId })
            .from(machine.table)
            .where(eq(machine.table.id, this.#options.machine));

        // wait for the row
        if (row === undefined) {
            return undefined;
        }
        // run a residency's cloud machine's spaces shared
        else if (row.kind === "cloud" && row.residencyId !== null) {
            return "shared";
        }
        // run an account's own cloud machine's spaces dedicated
        else if (row.kind === "cloud") {
            return "dedicated";
        }
        // run a person's computer's spaces on their own machine
        else {
            return "own";
        }
    }

    /** Read the tenancy a space asks for, and whether only the setting's default decides it. */
    static async #requested(
        database: DatabaseConnection,
        spaceId: Identifier<"space">,
    ): Promise<{ readonly tenancy: Tenancy; readonly isDefault: boolean }> {
        const { chain, values } = await SettingValue.along(database, [tenancy.reference], spaceId);
        const resolution = tenancy.resolve({ scope: spaceId }, values, chain);

        return {
            tenancy: resolution.value,
            isDefault: resolution.sources.every((source) => source.kind === "default"),
        };
    }

    /** Decide whether the account's entitlement unlocks a tenancy. */
    async #allowed(
        database: DatabaseConnection,
        accountId: Identifier<"account">,
        asked: Tenancy,
    ): Promise<Condition> {
        return !this.#options.isEnforcingEntitlements
            ? {
                  status: "true",
                  reason: "Unenforced",
                  message: "this machine enforces no entitlement",
              }
            : Entitlement.check(database, accountId, { tenancy: asked });
    }

    /** Find the machine that runs a tenancy for a space. */
    async #target(database: DatabaseConnection, row: Space, asked: Tenancy): Promise<Target> {
        // find a cloud machine serving the account's shared spaces
        if (asked === "shared") {
            return this.#cloudMachine(database, row);
        }

        // find the account's one enabled machine of the tenancy's kind
        const kind = asked === "dedicated" ? "cloud" : "personal";
        const machines = await database
            .select({ id: machine.table.id })
            .from(machine.table)
            .where(
                and(
                    eq(machine.table.scope, row.scope),
                    eq(machine.table.kind, kind),
                    eq(machine.table.status, "enabled"),
                    isNull(machine.table.revokedAt),
                ),
            )
            .orderBy(machine.table.id);
        const [only] = machines;
        // refuse an account without such a machine
        if (only === undefined) {
            const message = `the account has no ${kind} machine`;

            return {
                kind: "refused",
                condition: { status: "false", reason: "NoMachine", message },
            };
        }
        // refuse choosing among several
        else if (machines.length > 1) {
            const listed = machines.map((each) => each.id).join(", ");
            const message = `transfer the space to one of the account's machines: ${listed}`;

            return {
                kind: "refused",
                condition: { status: "false", reason: "Ambiguous", message },
            };
        }

        return { kind: "machine", machine: only.id };
    }

    /** Read the residency of an account from its replica. */
    static async #residency(
        database: DatabaseConnection,
        accountId: Identifier<"account">,
    ): Promise<ResidencyCode> {
        const [owner] = await database
            .select({ residencyId: account.table.residencyId })
            .from(account.table)
            .where(eq(account.table.id, accountId));

        return ResidencyCode.parse(present(owner, `the replica of ${accountId}`).residencyId);
    }

    /** Pick the cloud machine of the account's residency scoring highest for the space. */
    async #cloudMachine(database: DatabaseConnection, row: Space): Promise<Target> {
        // list the cloud machines of the account's residency
        const residency = await Scheduler.#residency(database, row.scope);
        const machines = await this.#options.accounts.residency.machines({ id: residency });

        // pick the machine scoring highest for the space
        const picked = await Placement.pick(
            row.id,
            machines.map((endpoint) => endpoint.machine),
        );

        return picked === undefined
            ? {
                  kind: "refused",
                  condition: {
                      status: "false",
                      reason: "NoCloudMachine",
                      message: `no cloud machine serves ${residency}`,
                  },
              }
            : { kind: "machine", machine: picked };
    }
}
