import type { Resource } from "../resource/resource.ts";
import { space } from "@destack/account/object";
import type { Snapshot } from "@destack/db";
import { Call, type ObservationReport, type ResourceType, type ResourceRow } from "@destack/object";
import { Address, Plan } from "@destack/resource";
import { principal } from "@destack/access";
import { ObjectReference } from "@destack/sync";
import { canonicalize, present, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { binding, type Installation, installation } from "../../object/index.ts";

/** One waiting plan and the object accepting it. */
type PendingPlan =
    | {
          /** An installation's own plan. */
          readonly kind: "installation";
          /** The installation accepting its own plan. */
          readonly object: typeof installation;
          /** The installation. */
          readonly id: string;
          /** The plan as its controller planned it. */
          readonly plan: Plan;
      }
    | {
          /** A resource's plan. */
          readonly kind: "resource";
          /** The resource kind accepting the plan. */
          readonly object: ResourceType<typeof space>;
          /** The resource. */
          readonly id: string;
          /** The address the plan's steps join under. */
          readonly address: string;
          /** The plan as its controller planned it. */
          readonly plan: Plan;
      };

/** The plans an installation and what it manages, owns or binds wait on, approved together. */
export class Approval {
    /** The waiting plans in the order the installations were read. */
    readonly waiting: readonly PendingPlan[];

    /** Collect waiting plans. */
    constructor(waiting: readonly PendingPlan[]) {
        this.waiting = waiting;
    }

    /** Read the plans an installation and the installations it manages wait on. */
    static async of(
        view: Pick<Snapshot, "read">,
        resources: readonly ResourceType<typeof space>[],
        target: Installation,
    ): Promise<Approval> {
        // walk the installation and those it manages and take each resource's plan once
        const waiting: PendingPlan[] = [];
        const seen = new Set<string>();
        const walked = new Set<string>();
        const queue: Installation[] = [target];
        for (const entry of queue) {
            // refuse a manager cycle, which the managed installations' tree forbids
            if (walked.has(entry.id)) {
                throw new TypeError(`installation ${entry.id} manages itself through a cycle`);
            }
            walked.add(entry.id);

            // take its own waiting plan
            if (entry.plan !== null && entry.conditions["ready"]?.reason === "AwaitingApproval") {
                waiting.push({
                    kind: "installation",
                    object: installation,
                    id: entry.id,
                    plan: entry.plan,
                });
            }

            // take the waiting plans of the resources it manages, owns or binds
            for (const { object, row } of await readResources(view, resources, entry)) {
                const plan = row.plan;
                const isWaiting = row.conditions["ready"]?.reason === "AwaitingApproval";
                if (isWaiting && plan !== null && !seen.has(row.id)) {
                    seen.add(row.id);
                    const address = Address.join(object.name, row.name);
                    waiting.push({ kind: "resource", object, id: row.id, address, plan });
                }
            }

            // queue the installations it manages
            queue.push(
                ...(await view.read(installation.table).findMany({
                    where: {
                        scope: entry.scope,
                        managerInstallationId: entry.id,
                    },
                })),
            );
        }

        return new Approval(waiting);
    }

    /** Join the waiting plans into the one plan an approver reviews. */
    plan(): Plan {
        return {
            steps: this.waiting.flatMap((entry) =>
                entry.plan.steps.map((step) =>
                    entry.kind === "resource"
                        ? { ...step, target: Address.join(entry.address, step.target) }
                        : step,
                ),
            ),
        };
    }

    /** Accept each waiting plan on its object as the space, where the object lives. */
    async approve(reviewed: string, call: Pick<Call, "mutate" | "self">): Promise<Plan> {
        // require the plan waiting now
        const plan = this.plan();
        await Approval.require(plan.steps.length === 0 ? null : plan, reviewed);

        // accept each part as its controller planned it, as the space
        const approvals = await Promise.all(
            this.waiting.map(async (entry) =>
                Call.record(entry.object, "approvePlan", {
                    id: entry.id,
                    plan: await Plan.digest(entry.plan),
                }),
            ),
        );
        await call.mutate(call.self, approvals);

        return plan;
    }

    /** Require a plan waiting for approval as an approver reviewed it, answering its digest. */
    static async require(plan: Plan | null, reviewed: string): Promise<string> {
        // refuse approving without a waiting plan
        if (plan === null) {
            throw new CallError("CONFLICT", { message: "no plan waits for approval" });
        }

        // refuse a plan other than the reviewed one
        const digest = await Plan.digest(plan);
        if (digest !== reviewed) {
            throw new CallError("CONFLICT", {
                message: `plan ${digest} waits for approval, not ${reviewed}`,
            });
        }

        return digest;
    }

    /** Describe an installation's own plan waiting for approval. */
    static async waiting(
        snapshot: Pick<Snapshot, "read">,
        target: Installation,
        plan: Plan,
    ): Promise<ObservationReport | undefined> {
        // pass a plan below the space's threshold, approved as it stands, or resuming the approved plan it is part of
        const scoped = present(
            await snapshot.read(space.table).findFirst({ where: { id: target.scope } }),
            `space ${target.scope}`,
        );
        const digest = await Plan.digest(plan);
        if (
            !Plan.isAtLeast(plan, scoped.approval) ||
            target.approvedPlan === digest ||
            (await Approval.#isResumed(target, plan))
        ) {
            return undefined;
        }

        // describe it waiting
        const message = `approve plan ${digest} with ${plan.steps.length} steps`;

        return {
            conditions: { ready: { status: "false", reason: "AwaitingApproval", message } },
            fields: { plan },
        };
    }

    /** Decide whether a plan resumes an approved plan the installation kept, each step approved. */
    static async #isResumed(target: Installation, plan: Plan): Promise<boolean> {
        // require the kept plan to be the approved one
        const kept = target.plan;
        if (kept === null || target.approvedPlan !== (await Plan.digest(kept))) {
            return false;
        }

        // require each step among the approved ones
        const approved = new Set(kept.steps.map((step) => canonicalize(step)));

        return plan.steps.every((step) => approved.has(canonicalize(step)));
    }
}

/** Read the resources of every kind an installation manages, owns or binds. */
async function readResources(
    view: Pick<Snapshot, "read">,
    resources: readonly ResourceType<typeof space>[],
    target: Installation,
): Promise<{ readonly object: ResourceType<typeof space>; readonly row: ResourceRow }[]> {
    // read the targets of its bindings
    const bound = await view.read(binding.table).findMany({
        where: {
            scope: target.scope,
            installationId: target.id,
        },
    });
    const targets = bound.map((entry) => entry.target);

    // read each kind's resources it manages, owns or binds in name order
    const found: Resource[] = [];
    const owner = ObjectReference.columns(
        "owner",
        principal.installation.reference(target.scope, target.id),
    );
    for (const object of resources) {
        const targeted = targets.filter((id) => object.identifies(id));
        const rows = await view.read(object.table).findMany({
            where: {
                scope: target.scope,
                OR: [
                    { managerInstallationId: target.id },
                    owner,
                    ...(targeted.length === 0
                        ? []
                        : [{ id: { in: targeted.map((id) => schema.anyIdentifier().parse(id)) } }]),
                ],
            },
        });
        found.push(...rows.map((row) => ({ object, row })));
    }

    return found.toSorted((left, right) => left.row.name.localeCompare(right.row.name, "en"));
}
