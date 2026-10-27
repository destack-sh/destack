import { RequestId } from "@destack/sync";
import { assignment, placement, Space, space } from "@destack/account/object";
import {
    and,
    eq,
    inArray,
    isNotNull,
    type DatabaseConnection,
    type Table,
    Change,
    Condition,
} from "@destack/db";
import type { ReconcileController } from "@destack/db";
import { type Identifier } from "@destack/schema";
import type { Uplink } from "@destack/sync";
import { ACTIVE_BUILD_STATES, build, deployment, Deployment } from "../../object/index.ts";
import type { Environment } from "@destack/environment";

/** Give other machines with work in a space an assignment in the directory until it ends. */
export class AssignmentController implements ReconcileController {
    /** The controller's name in reports. */
    readonly name = "assignment";
    /** The tables of work addressed to machines, and the replicated placements and assignments. */
    readonly watches: readonly Table[] = [
        build.table,
        deployment.table,
        placement.table,
        assignment.table,
    ];
    /** The database of the spaces this machine serves, with their placements and assignments. */
    readonly #database: DatabaseConnection;
    /** The universe's uplink, which assigns the work as this machine. */
    readonly #publisher: Uplink;
    /** This machine. */
    readonly #machine: Identifier<"machine">;
    /** The environment the assignments are made in. */
    readonly #environment: Environment;

    /** Assign the work of a machine's spaces through the universe's uplink. */
    constructor(
        database: DatabaseConnection,
        publisher: Uplink,
        machine: Identifier<"machine">,
        environment: Environment,
    ) {
        // store the machine's database, the uplink it assigns through and its environment
        this.#database = database;
        this.#publisher = publisher;
        this.#machine = machine;
        this.#environment = environment;
    }

    /** List the space a change of its work, its placement or its assignments touches. */
    keys(change: Change): readonly string[] {
        // take a placement's space
        if (Change.of(change, placement.table)) {
            return [Change.image(change).id];
        }
        // take an assignment's space
        else if (Change.of(change, assignment.table)) {
            return [Change.image(change).placementId];
        }
        // take the space of its work otherwise
        else {
            return [change.scope];
        }
    }

    /** List the spaces this machine serves. */
    async list(): Promise<readonly string[]> {
        const rows = await this.#database
            .select({ id: space.table.id })
            .from(space.table)
            .where(Space.served());

        return rows.map((row) => row.id);
    }

    /** Assign the machines with work in a space this machine serves and withdraw the others. */
    async reconcile(key: string): Promise<undefined> {
        // act only for a space the replicated placement places on this machine
        const spaceId = space.identifier(key);
        const [placed] = await this.#database
            .select()
            .from(placement.table)
            .where(
                and(eq(placement.table.id, spaceId), eq(placement.table.machineId, this.#machine)),
            );
        if (placed === undefined) {
            return undefined;
        }

        // assign the other machines with active builds of their checkouts or live deployments
        const working = new Set(await this.#machines(spaceId));
        working.delete(placed.machineId);
        const assigned = new Set(await this.#assigned(spaceId));
        const work = (name: "assign" | "unassign", machine: Identifier<"machine">) =>
            placement.call(name, {
                accountId: placed.scope,
                id: placed.id,
                epoch: placed.epoch,
                machine,
            });

        // assign the new work and withdraw the ended work in one mutation
        const calls = [
            ...[...working.difference(assigned)].map((machine) => work("assign", machine)),
            ...[...assigned.difference(working)].map((machine) => work("unassign", machine)),
        ];
        if (calls.length > 0) {
            await this.#publisher.receive({
                id: RequestId.create(this.#environment),
                calls,
            });
        }

        return undefined;
    }

    /** List the machines a space gave work, from the replicated assignments. */
    async #assigned(spaceId: Identifier<"space">): Promise<Identifier<"machine">[]> {
        const rows = await this.#database
            .select({ machine: assignment.table.machineId })
            .from(assignment.table)
            .where(eq(assignment.table.placementId, spaceId));

        return rows.map((row) => row.machine);
    }

    /** List the machines a space's active builds and live deployments are addressed to. */
    async #machines(scope: Identifier<"space">): Promise<Identifier<"machine">[]> {
        const [builds, deployments] = await Promise.all([
            this.#database
                .select({ source: build.table.source })
                .from(build.table)
                .where(
                    and(
                        eq(build.table.scope, scope),
                        inArray(build.table.state, [...ACTIVE_BUILD_STATES]),
                    ),
                ),
            this.#database
                .select({ machineId: deployment.table.machineId })
                .from(deployment.table)
                .where(
                    and(
                        eq(deployment.table.scope, scope),
                        Condition.render(Deployment.live(), deployment.table),
                        isNotNull(deployment.table.machineId),
                    ),
                ),
        ]);

        return [
            ...builds.flatMap((row) =>
                row.source.kind === "checkout" ? [row.source.machine] : [],
            ),
            ...deployments.flatMap((row) => (row.machineId === null ? [] : [row.machineId])),
        ];
    }
}
