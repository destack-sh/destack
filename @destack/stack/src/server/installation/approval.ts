import { space } from "@destack/account/object";
import { Snapshot } from "@destack/db";
import {
    approval,
    AWAITING_APPROVAL,
    changeRequest,
    type ChangeRequest,
    installation,
    MethodContext,
    type ObjectReconciliation,
    type ObservationReport,
    type ResourceType,
    type Resource,
} from "@destack/object";
import { Address, Plan } from "@destack/resource";
import { principal } from "@destack/access";
import { ObjectReference, ObjectTypeReference, Scope, type Subject } from "@destack/sync";
import type { Crypto } from "@destack/environment";
import { canonicalize, Digest, type Identifier, present, schema } from "@destack/schema";
import { MAIN_DATABASE } from "../../declare/index.ts";
import {
    Database,
    installationRevision,
    type InstallationRollout,
    Rollout,
    rollout,
} from "../../object/index.ts";
import type {} from "@destack/package/import-meta";
import { StackError } from "../../error/index.ts";

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

/** The input of the call a change request makes to approve a plan: the plan's digest. */
const RequestedPlan = schema.looseObject({ plan: Digest });

/** The plans an installation and its managed, owned or bound resources wait on. */
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
        target: InstallationRollout,
    ): Promise<Approval> {
        // walk the installation and those it manages and take each resource's plan once
        const waiting: PendingPlan[] = [];
        const seen = new Set<string>();
        const walked = new Set<string>();
        const queue: InstallationRollout[] = [target];
        for (const entry of queue) {
            // refuse a manager cycle, which the managed installations' tree forbids
            if (walked.has(entry.installationId)) {
                throw new TypeError(
                    `installation ${entry.installationId} manages itself through a cycle`,
                );
            }
            walked.add(entry.installationId);

            // take its own waiting plan
            const installed = entry.installation;
            const isAwaiting = installed.conditions["Ready"]?.reason === AWAITING_APPROVAL;
            if (installed.plan !== null && isAwaiting) {
                const { plan } = installed;
                waiting.push({
                    kind: "installation",
                    object: installation,
                    id: entry.installationId,
                    plan,
                });
            }

            // take the waiting plans of its managed, owned or bound resources
            for (const { object, row } of await readResources(view, resources, entry)) {
                const plan = row.plan;
                const isWaiting = row.conditions["Ready"]?.reason === AWAITING_APPROVAL;
                if (isWaiting && plan !== null && !seen.has(row.id)) {
                    seen.add(row.id);
                    const address = Address.join(object.name, row.name);
                    waiting.push({ kind: "resource", object, id: row.id, address, plan });
                }
            }

            // queue the installations it manages
            const managed = await view.read(installation.table).findMany({
                where: { scope: entry.scope, managerInstallationId: entry.installationId },
            });
            queue.push(...(await Rollout.installed(view, managed)));
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

    /** Keep one change request open for the plan an installation waits on, failing if rejected. */
    async request(
        rolled: Pick<Rollout, "id" | "scope" | "installationId">,
        reconciliation: Pick<ObjectReconciliation, "database" | "mutate" | "store">,
    ): Promise<void> {
        // read the plan waiting and its digest, none when nothing waits
        const plan = this.plan();
        const { crypto } = reconciliation.store.environment;
        const digest = plan.steps.length === 0 ? undefined : await Plan.digest(plan, crypto);

        // read the rollout's plan approvals still open or rejected, and the plan each asks for
        const parent = { packageId: rollout.package.id, type: rollout.name, id: rolled.id };
        const requests = await Approval.requests(Snapshot.live(reconciliation.database), rolled);
        const asked = (row: (typeof requests)[number]) => RequestedPlan.parse(row.call.input).plan;

        // withdraw the open requests for another plan, and ask for this one unless decided already
        const stale = requests.filter((row) => row.state === "open" && asked(row) !== digest);
        const isAsked = requests.some((row) => asked(row) === digest);
        const managers = Approval.managers(rolled);
        const opened =
            digest === undefined || isAsked
                ? []
                : [
                      changeRequest.call("create", {
                          ...changeRequest.in(rolled.scope),
                          parent,
                          call: rollout.call("approve", {
                              ...rollout.target({ scope: rolled.scope, id: rolled.installationId }),
                              plan: digest,
                          }),
                          ...ObjectReference.columns("as", managers),
                          ...(managers.relation === undefined
                              ? {}
                              : { asRelation: managers.relation }),
                      }),
                  ];
        const calls = [
            ...stale.map((row) => changeRequest.call("withdraw", changeRequest.target(row))),
            ...opened,
        ];
        if (calls.length > 0) {
            await reconciliation.mutate(calls);
        }

        // fail the pass on a rejected plan until the plan changes
        const rejected = requests.find((row) => row.state === "rejected" && asked(row) === digest);
        if (rejected !== undefined) {
            throw StackError.of(
                "PLAN_REJECTED",
                `plan ${digest} was rejected: change the installation for another plan`,
            );
        }
    }

    /** Read a rollout's plan approvals still open or rejected, past its other change requests. */
    static async requests(
        reader: Pick<Snapshot, "read">,
        rolled: Pick<Rollout, "id" | "scope">,
    ): Promise<ChangeRequest["row"][]> {
        const found = await reader.read(changeRequest.table).findMany({
            where: {
                scope: rolled.scope,
                parentPackageId: rollout.package.id,
                parentType: rollout.name,
                parentId: rolled.id,
                state: { in: ["open", "rejected"] },
            },
        });
        const approving = ObjectTypeReference.key({
            packageId: rollout.package.id,
            type: rollout.name,
        });

        return found.filter(
            (row) =>
                row.call.method === "approve" &&
                ObjectTypeReference.key(row.call.type) === approving,
        );
    }

    /** Read the approval a plan's open change request asks, absent before the request opens. */
    static async asked(
        reader: Pick<Snapshot, "read">,
        rolled: Pick<Rollout, "id" | "scope">,
        digest: string,
    ): Promise<Identifier<"approval"> | null> {
        // find the open request for the plan, then the approval it asks
        const requests = await Approval.requests(reader, rolled);
        const request = requests.find(
            (row) => row.state === "open" && RequestedPlan.parse(row.call.input).plan === digest,
        );
        const asked =
            request === undefined
                ? undefined
                : await reader
                      .read(approval.table)
                      .findFirst({ where: { scope: rolled.scope, parentId: request.id } });

        return asked?.id ?? null;
    }

    /** Read who a plan asks: the managers of the installation's space, as one subject set. */
    static managers(target: Pick<Rollout, "scope">): Subject {
        return {
            ...principal.space.reference(Scope.universe.id, target.scope),
            relation: "manage",
        };
    }

    /** Accept each waiting plan on its object as the space, where the object lives. */
    async approve(
        reviewed: string,
        context: Pick<MethodContext, "mutate" | "self" | "environment">,
    ): Promise<Plan> {
        // require the plan waiting now
        const plan = this.plan();
        const { crypto } = context.environment;
        await Approval.require(plan.steps.length === 0 ? null : plan, reviewed, crypto);

        // accept each part as its controller planned it, as the space
        const approvals = await Promise.all(
            this.waiting.map(async (entry) =>
                entry.object.call("approvePlan", {
                    id: entry.id,
                    plan: await Plan.digest(entry.plan, crypto),
                }),
            ),
        );
        await context.mutate(context.self, approvals);

        return plan;
    }

    /** Require a plan waiting for approval as an approver reviewed it, answering its digest. */
    static async require(
        plan: Plan | null,
        reviewed: string,
        crypto: Pick<Crypto, "digest">,
    ): Promise<string> {
        // refuse approving without a waiting plan
        if (plan === null) {
            throw StackError.of("PLAN_WAITING_MISSING", "no plan is waiting for approval");
        }

        // refuse a plan other than the reviewed one
        const digest = await Plan.digest(plan, crypto);
        if (digest !== reviewed) {
            throw StackError.of(
                "PLAN_MISMATCH",
                `plan ${digest} is waiting for approval, not ${reviewed}`,
            );
        }

        return digest;
    }

    /** Describe an installation's own plan waiting for approval. */
    static async waiting(
        snapshot: Pick<Snapshot, "read">,
        target: InstallationRollout,
        plan: Plan,
        crypto: Pick<Crypto, "digest">,
    ): Promise<ObservationReport | undefined> {
        // pass a plan below the space's threshold, approved as it stands, or resuming the approved plan it is part of
        const scoped = present(
            await snapshot.read(space.table).findFirst({ where: { id: target.scope } }),
            `space ${target.scope}`,
        );
        const digest = await Plan.digest(plan, crypto);
        if (
            !Plan.isAtLeast(plan, scoped.approval) ||
            target.installation.approvedPlan === digest ||
            (await Approval.#isResumed(target, plan, crypto))
        ) {
            return undefined;
        }

        // describe it waiting
        const message = `approve plan ${digest} with ${plan.steps.length} steps`;

        return {
            conditions: { Ready: { status: "false", reason: AWAITING_APPROVAL, message } },
            fields: { plan },
        };
    }

    /** Decide whether a plan resumes an approved plan of the installation, each step approved. */
    static async #isResumed(
        target: InstallationRollout,
        plan: Plan,
        crypto: Pick<Crypto, "digest">,
    ): Promise<boolean> {
        // require the stored plan to be the approved one
        const { plan: kept, approvedPlan } = target.installation;
        if (kept === null || approvedPlan !== (await Plan.digest(kept, crypto))) {
            return false;
        }

        // require each step among the approved ones
        const approved = new Set(kept.steps.map((step) => canonicalize(step)));

        return plan.steps.every((step) => approved.has(canonicalize(step)));
    }
}

/** Read the resources an installation manages, owns or binds, main when it keeps tables there. */
async function readResources(
    view: Pick<Snapshot, "read">,
    resources: readonly ResourceType<typeof space>[],
    target: InstallationRollout,
): Promise<{ readonly object: ResourceType<typeof space>; readonly row: Resource }[]> {
    // read the targeted revision and the targets here it binds
    const revision =
        target.revisionId === null
            ? undefined
            : await view
                  .read(installationRevision.table)
                  .findFirst({ where: { id: target.revisionId } });
    const bound = revision === undefined ? [] : revision.bindings;
    const main =
        revision?.states[MAIN_DATABASE] === undefined
            ? undefined
            : await Database.mainMaybe(view, schema.identifier("space").parse(target.scope));
    const targets = [
        ...bound.map((entry) => entry.target),
        ...(main === undefined ? [] : [main.id]),
    ];

    // read each kind's managed, owned or bound resources in name order
    const found: { readonly object: ResourceType<typeof space>; readonly row: Resource }[] = [];
    const owner = ObjectReference.columns(
        "owner",
        principal.installation.reference(target.scope, target.installationId),
    );
    for (const object of resources) {
        const targeted = targets.filter((id) => object.identifies(id));
        const rows = await view.read(object.table).findMany({
            where: {
                scope: target.scope,
                OR: [
                    { managerInstallationId: target.installationId },
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
