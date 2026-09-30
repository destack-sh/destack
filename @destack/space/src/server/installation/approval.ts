import { and, type DatabaseConnection, eq, inArray, or } from "@destack/db";
import type { ObjectReconciliation } from "@destack/object";
import { SystemCall } from "@destack/object/server";
import { Address, Plan } from "@destack/resource";
import { ServiceError } from "@destack/service/error";
import {
    binding,
    type Installation,
    installation,
    type Resource,
    resource,
    space,
} from "../../object/index.ts";
import type { Invoke } from "./stack.ts";

/** One waiting plan and the object accepting it. */
interface Waiting {
    /** The object type accepting the plan. */
    readonly object: typeof installation | typeof resource;
    /** The accepting object. */
    readonly id: string;
    /** The address the plan's steps join under, empty for an installation's own. */
    readonly address: string;
    /** The plan as its controller planned it. */
    readonly plan: Plan;
}

/** The plans an installation and what it manages, owns or binds wait on, approved together. */
export class Approval {
    /** The waiting plans in the order the installations were read. */
    readonly waiting: readonly Waiting[];

    /** Collect waiting plans. */
    constructor(waiting: readonly Waiting[]) {
        this.waiting = waiting;
    }

    /** Read the plans an installation, the installations it manages, and their resources wait on. */
    static async of(database: DatabaseConnection, target: Installation): Promise<Approval> {
        // walk the installation and those it manages, taking each resource's plan once
        const waiting: Waiting[] = [];
        const seen = new Set<string>();
        const queue: Installation[] = [target];
        for (let index = 0; index < queue.length; index += 1) {
            // take its own waiting plan
            const entry = queue[index]!;
            if (entry.plan !== null) {
                waiting.push({ object: installation, id: entry.id, address: "", plan: entry.plan });
            }

            // take the waiting plans of the resources it manages, owns or binds
            for (const row of await resourcesOf(database, entry)) {
                const plan = row.status.state?.plan;
                const isWaiting = row.conditions.ready?.reason === "AwaitingApproval";
                if (isWaiting && plan !== undefined && !seen.has(row.id)) {
                    seen.add(row.id);
                    const address = Address.join("resource", row.name);
                    waiting.push({ object: resource, id: row.id, address, plan });
                }
            }

            // queue the installations it manages
            queue.push(
                ...(await database
                    .select()
                    .from(installation.table)
                    .where(
                        and(
                            eq(installation.table.scope, entry.scope),
                            eq(installation.table.managerInstallationId, entry.id),
                        ),
                    )),
            );
        }

        return new Approval(waiting);
    }

    /** Join the waiting plans into the one plan an approver reviews. */
    plan(): Plan {
        return {
            steps: this.waiting.flatMap(({ address, plan }) =>
                plan.steps.map((step) =>
                    address === "" ? step : { ...step, target: Address.join(address, step.target) },
                ),
            ),
        };
    }

    /** Accept each waiting plan on its object once the reviewed digest matches the joined plan. */
    async grant(reviewed: string, invoke: Invoke): Promise<Plan> {
        // require the plan waiting now
        const plan = this.plan();
        const digest = await Plan.digest(plan);
        if (plan.steps.length === 0) {
            throw new ServiceError("CONFLICT", { message: "no plan waits for approval" });
        } else if (digest !== reviewed) {
            throw new ServiceError("CONFLICT", {
                message: `plan ${digest} waits for approval, not ${reviewed}`,
            });
        }

        // accept each part as its controller planned it
        for (const entry of this.waiting) {
            await invoke(entry.object, "accept", {
                id: entry.id,
                plan: await Plan.digest(entry.plan),
            });
        }

        return plan;
    }

    /** Report whether an installation's own plan waits for approval, observing it waiting when it does. */
    static async awaits(
        target: Installation,
        plan: Plan,
        reconciliation: ObjectReconciliation,
    ): Promise<boolean> {
        // pass a plan below the space's threshold or approved as it stands
        const [scoped] = await reconciliation.database
            .select({ approval: space.table.approval })
            .from(space.table)
            .where(eq(space.table.id, target.scope));
        const digest = await Plan.digest(plan);
        if (!Plan.reaches(plan, scoped!.approval) || target.approvedPlan === digest) {
            return false;
        }

        // observe it waiting unless it does already
        const message = `approve plan ${digest} with ${plan.steps.length} steps`;
        const ready = target.conditions.ready;
        const isObserved =
            ready?.reason === "AwaitingApproval" &&
            ready.message === message &&
            ready.observedGeneration === target.generation;
        if (!isObserved) {
            await reconciliation.server.executeAsSystem(
                installation,
                "observe",
                [
                    SystemCall.of(target, {
                        observedGeneration: target.generation,
                        conditions: {
                            ready: { status: "false", reason: "AwaitingApproval", message },
                        },
                        fields: { plan },
                    }),
                ],
                reconciliation.now,
            );
        }

        return true;
    }
}

/** Read the resources an installation manages, owns or binds. */
async function resourcesOf(
    database: DatabaseConnection,
    target: Installation,
): Promise<Resource[]> {
    // read the targets of its bindings
    const bound = await database
        .select({ target: binding.table.target })
        .from(binding.table)
        .where(
            and(eq(binding.table.scope, target.scope), eq(binding.table.installationId, target.id)),
        );
    const targets = bound.map((entry) => entry.target);

    return database
        .select()
        .from(resource.table)
        .where(
            and(
                eq(resource.table.scope, target.scope),
                or(
                    eq(resource.table.managerInstallationId, target.id),
                    eq(resource.table.ownerInstallationId, target.id),
                    targets.length === 0 ? undefined : inArray(resource.table.id, targets),
                ),
            ),
        )
        .orderBy(resource.table.name);
}
