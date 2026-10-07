import { accessRelationship } from "@destack/access";
import { and, type DatabaseConnection, eq, Snapshot } from "@destack/db";
import {
    Address,
    Plan,
    type Provider,
    type ResourceKind,
    type Risk,
    type Step,
} from "@destack/resource";
import { PlanError } from "@destack/resource/error";
import type { ResourceState } from "@destack/package/declare";
import { aligned, canonicalize, Digest, Duration, type Identifier, present } from "@destack/schema";
import { Replica, Scope } from "@destack/sync";
import { SystemCall } from "../method/system.ts";
import {
    ObjectWatch,
    type ObjectController,
    type ObjectReconciliation,
} from "../object/controller.ts";
import type { Condition } from "../object/condition.ts";
import { type ObjectType } from "../object/object.ts";
import { CONSUMER } from "../trait/bindable.ts";
import { type ProvisionedObject, type ProvisionedRecord } from "../trait/provisioned.ts";

/** The condition a resource reports while it waits for its draining consumers to stop before a recreation. */
export const DRAINING = "draining";

/** What a process reconciling the resources it serves supplies: its providers and its machine. */
export interface ProvisionedOptions {
    /** The providers the process holds, in preference order within each kind. */
    readonly providers: readonly Pick<
        Provider<ResourceKind, ObjectType>,
        "kind" | "code" | "object" | "provision" | "reconcile" | "fence" | "snapshot"
    >[];
    /** The machine keeping the resources its providers provision, absent for a region. */
    readonly machine: Identifier<"machine"> | null;
}

/** The control loop of one kind's provisioned objects: it provisions, observes, plans, applies with approval and retires them through the providers a process holds. */
export class ProvisionedController implements ObjectController<ProvisionedObject<"space">> {
    /** The rows the loop takes: every resource of the kind. */
    readonly pending = {};
    /** The consumer relationships, whose changes select the resource they relate. */
    readonly watches: readonly ObjectWatch[];
    /** The provisioned object type. */
    readonly #object: ProvisionedObject<"space">;
    /** The process's providers of the kind and its machine. */
    readonly #options: ProvisionedOptions;

    /** Reconcile one kind's provisioned objects through the providers of its kind, refusing an object with a controller of its own. */
    constructor(object: ProvisionedObject<"space">, options: ProvisionedOptions) {
        // refuse resources another controller reconciles
        if (object.controller !== undefined) {
            throw new TypeError(`resources ${object.name} take their controller from their kind`);
        }

        // keep the providers of the object's kind
        const providers = options.providers.filter((provider) => provider.object.same(object));
        if (providers.length === 0) {
            throw new TypeError(`no provider here serves ${object.name} resources`);
        }
        this.#object = object;
        this.#options = { ...options, providers };

        // wake a resource once its consumers change, and the resources of a scope this database starts or stops holding
        const { packageId } = object.policy.definition;
        this.watches = [
            ObjectWatch.of(accessRelationship, (row) =>
                row.packageId === packageId && row.type === object.name && row.relation === CONSUMER
                    ? [{ id: row.objectId }]
                    : [],
            ),
            ObjectWatch.of(Scope.table, async (row, database) =>
                object.scopes.some(
                    (scope) =>
                        scope.typeReference.packageId === row.packageId &&
                        scope.typeReference.type === row.type,
                )
                    ? database
                          .select({ id: object.table.id })
                          .from(object.table)
                          .where(object.inScope(row.scope))
                    : [],
            ),
        ];
    }

    /** Reconcile one resource, returning when to look again. */
    reconcile(reconciliation: ObjectReconciliation<ProvisionedObject<"space">>) {
        return reconcile(
            this.#object,
            aligned(reconciliation.rows, 0),
            reconciliation,
            this.#options,
        );
    }

    /** Select the provider a resource requests, or the first of its kind. */
    static select<Selected extends Pick<ProvisionedOptions["providers"][number], "code">>(
        providers: readonly Selected[],
        row: Pick<ProvisionedRecord, "provider" | "placement">,
    ): Selected | undefined {
        const requested = row.provider ?? row.placement?.provider ?? null;

        return requested === null
            ? providers[0]
            : providers.find((provider) => provider.code === requested);
    }
}

/** Reconcile one resource: provision, retire, and apply its desired states, returning when to look again. */
async function reconcile(
    object: ProvisionedObject<"space">,
    row: ProvisionedRecord,
    reconciliation: ObjectReconciliation<ProvisionedObject<"space">>,
    options: ProvisionedOptions,
): Promise<number | undefined> {
    // act only on a resource whose scope this database holds and whose owner set its approval threshold
    const { approval } = row;
    if (approval === null || !(await isHeld(reconciliation.database, row.scope))) {
        return undefined;
    }

    // retire a resource whose deletion was requested
    if (row.deletionRequestedAt !== null) {
        return retire(object, row, row.deletionRequestedAt, reconciliation, options);
    }
    // observe a resource lent from a connection ready at its own identifier below the host's egress
    else if (row.origin === "connection") {
        await observe(
            object,
            row,
            reconciliation,
            { reference: row.id },
            {
                ready: {
                    status: "true",
                    reason: "Lent",
                    message:
                        row.connection === null
                            ? "lent from the connections each call names"
                            : `lent from connection ${row.connection}`,
                },
            },
        );
    }
    // observe a resource whose stack declares its reference ready as declared
    else if (row.origin === "declared") {
        await observe(
            object,
            row,
            reconciliation,
            {},
            {
                ready: {
                    status: "true",
                    reason: "Declared",
                    message: `declared at ${row.reference}`,
                },
            },
        );
    }
    // provision a resource whose specification changed and apply its desired states
    else if (row.generation > row.observedGeneration || row.reference === null) {
        const provisioned = await provision(object, row, reconciliation, options);
        if (provisioned) {
            await apply(object, provisioned, approval, reconciliation, options);
        }
    }
    // apply the desired states of a provisioned resource
    else {
        await apply(object, row, approval, reconciliation, options);
    }

    return undefined;
}

/** Decide whether a database holds a scope that no transfer fences or copies here. */
async function isHeld(database: DatabaseConnection, scope: string): Promise<boolean> {
    const [link] = await Scope.chain(Snapshot.live(database), scope);

    return (
        link !== undefined &&
        link.movedTo === undefined &&
        !(await Replica.isCopied(database, scope))
    );
}

/** Provision a resource through its provider, recording the outcome and returning the provisioned row. */
async function provision(
    object: ProvisionedObject<"space">,
    row: ProvisionedRecord,
    reconciliation: ObjectReconciliation<ProvisionedObject<"space">>,
    options: ProvisionedOptions,
): Promise<ProvisionedRecord | undefined> {
    // select the requested provider or the first one providing the kind
    const provider = ProvisionedController.select(options.providers, row);
    if (provider === undefined) {
        const message = `no provider for ${object.name} ${row.name}`;
        await observe(object, row, reconciliation, {}, { ready: unready("NoProvider", message) });

        return undefined;
    }
    // refuse a provider provisioning nothing
    else if (provider.provision === undefined) {
        const message = `provider ${provider.code} of ${object.name} provisions nothing`;
        await observe(
            object,
            row,
            reconciliation,
            {},
            {
                ready: unready("NoProvisioning", message),
            },
        );

        return undefined;
    }

    // record the provider reference for this generation or the failure before retrying it
    try {
        const provisioned = await provider.provision.provision(provider.kind.record(row));
        const fields = {
            provider: provider.code,
            reference: provisioned.reference,
            location: provisioned.location ?? null,
            machineId: options.machine,
        };

        return await observe(object, row, reconciliation, fields, {
            ready: unready("Provisioned", provisioned.reference),
        });
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        await observe(
            object,
            row,
            reconciliation,
            {},
            {
                ready: unready("ProvisioningFailed", message),
            },
        );
        throw error;
    }
}

/** Plan a provisioned resource toward its desired states and apply what its approval threshold or an approval allows. */
async function apply(
    object: ProvisionedObject<"space">,
    row: ProvisionedRecord,
    approval: Risk,
    reconciliation: ObjectReconciliation<ProvisionedObject<"space">>,
    options: ProvisionedOptions,
): Promise<void> {
    // require the resource's provider
    const provider = ProvisionedController.select(options.providers, row);
    if (provider === undefined) {
        const message = `no provider for ${object.name} ${row.name}`;
        await observe(object, row, reconciliation, {}, { ready: unready("NoProvider", message) });

        return;
    }

    // skip resources whose desired states applied already
    const desired = [...row.states, ...row.drainingStates];
    const stateDigest = await Digest.json(desired);
    if (row.appliedState === stateDigest && row.conditions["ready"]?.reason === "Applied") {
        return;
    }

    // plan the desired states or a recreation stopping the draining consumers first
    const planned = await planRecreation(object, row, provider, reconciliation);
    if (planned === undefined) {
        return;
    }
    const { plan, isDraining } = planned;
    const planDigest = await Plan.digest(plan);
    const waiting = { steps: plan.steps.map(planStep) };

    // wait for an approval of this exact plan when its risk reaches the approval threshold
    if (Plan.isAtLeast(plan, approval) && row.approvedPlan !== planDigest) {
        const message = `approve plan ${planDigest} with ${plan.steps.length} steps`;
        await observe(
            object,
            row,
            reconciliation,
            { plan: waiting },
            {
                ready: unready("AwaitingApproval", message),
            },
        );
    }
    // report draining for the consumers' owner to stop them
    else if (isDraining) {
        const message = "stopping the draining consumers before applying the plan";
        await observe(
            object,
            row,
            reconciliation,
            { plan: waiting, approvedPlan: null },
            {
                ready: unready("Recreating", message),
                [DRAINING]: { status: "true", reason: "Recreating", message },
            },
        );
    }
    // apply the plan and record the desired states it applied
    else {
        const digests = { plan: planDigest, state: stateDigest };
        await applyPlan(object, row, provider, plan, digests, reconciliation);
    }
}

/** Plan a resource's desired states, or a recreation without the states only draining consumers need, absent when blocked. */
async function planRecreation(
    object: ProvisionedObject<"space">,
    row: ProvisionedRecord,
    provider: ProvisionedOptions["providers"][number],
    reconciliation: ObjectReconciliation<ProvisionedObject<"space">>,
): Promise<{ readonly plan: Plan; readonly isDraining: boolean } | undefined> {
    // plan the desired states
    const planOf = (states: readonly ResourceState[]) =>
        provider.reconcile !== undefined
            ? provider.reconcile.plan(provider.kind.record(row), provider.kind.states(states))
            : Promise.resolve({ steps: [] });
    try {
        return { plan: await planOf([...row.states, ...row.drainingStates]), isDraining: false };
    } catch (error) {
        // plan without the states only draining consumers need
        const recreated =
            error instanceof PlanError && row.drainingStates.length > 0
                ? await planOf(row.states).catch((failure: unknown) => {
                      // block when the active consumers alone conflict too
                      if (failure instanceof PlanError) {
                          return undefined;
                      }
                      throw failure;
                  })
                : undefined;

        // block on what the declarations must fix and retry another failure
        if (recreated === undefined) {
            const isBlocked = error instanceof PlanError;
            const message = error instanceof Error ? error.message : String(error);
            const reason = isBlocked ? "Blocked" : "PlanFailed";
            await observe(object, row, reconciliation, {}, { ready: unready(reason, message) });
            if (!isBlocked) {
                throw error;
            }

            return undefined;
        }

        // stop the draining consumers before the recreation
        const stop: Step = {
            action: "delete",
            target: Address.join("consumer", DRAINING),
            risk: "backward-incompatible",
            detail: `stop the consumers needing ${row.drainingStates.length} draining states`,
        };

        return { plan: { steps: [stop, ...recreated.steps] }, isDraining: true };
    }
}

/** Apply a plan through the resource's provider and record the desired states it applied. */
async function applyPlan(
    object: ProvisionedObject<"space">,
    row: ProvisionedRecord,
    provider: ProvisionedOptions["providers"][number],
    plan: Plan,
    digests: { readonly plan: string; readonly state: string },
    reconciliation: ObjectReconciliation<ProvisionedObject<"space">>,
): Promise<void> {
    try {
        // apply the plan
        if (provider.reconcile !== undefined) {
            await provider.reconcile.apply(
                provider.kind.record(row),
                provider.kind.states([...row.states, ...row.drainingStates]),
                digests.plan,
            );
        }

        // record the desired states it applied and the end of a drain
        const message = `${plan.steps.length} steps applied`;
        await observe(
            object,
            row,
            reconciliation,
            { appliedState: digests.state, plan: null, approvedPlan: null },
            {
                ready: { status: "true", reason: "Applied", message },
                ...(row.conditions[DRAINING] === undefined
                    ? {}
                    : { [DRAINING]: { status: "false", reason: "Applied", message } }),
            },
        );
    } catch (error) {
        // record the failure and keep the plan
        const message = error instanceof Error ? error.message : String(error);
        const waiting = { steps: plan.steps.map(planStep) };
        await observe(
            object,
            row,
            reconciliation,
            { plan: waiting },
            {
                ready: unready("ApplyFailed", message),
            },
        );
        throw error;
    }
}

/** Remove an unused resource, destroying its content and returning when its retention ends. */
async function retire(
    object: ProvisionedObject<"space">,
    row: ProvisionedRecord,
    requestedAt: number,
    reconciliation: ObjectReconciliation<ProvisionedObject<"space">>,
    options: ProvisionedOptions,
): Promise<number | undefined> {
    // keep the record and content of retained resources until owned again or their window ends
    const retention = row.retention;
    const ends =
        typeof retention === "object"
            ? requestedAt + Duration.milliseconds(retention.within)
            : undefined;
    if (retention === "forever" || (ends !== undefined && ends > reconciliation.now)) {
        if (row.conditions["ready"]?.reason !== "Retained") {
            const message = "content retained after removal";
            await observe(
                object,
                row,
                reconciliation,
                {},
                {
                    ready: unready("Retained", message),
                },
            );
        }

        return ends === undefined ? undefined : ends - reconciliation.now;
    }

    // wait until no consumer uses the resource
    const consumer = await consumerOf(object, row, reconciliation.database);
    if (consumer !== undefined) {
        const message = `consumed by ${consumer}`;
        await observe(object, row, reconciliation, {}, { ready: unready("InUse", message) });

        return undefined;
    }

    // destroy provisioned content and record a failure before retrying it
    const provider = ProvisionedController.select(options.providers, row);
    if (row.reference !== null && provider?.provision !== undefined) {
        try {
            await provider.provision.destroy(provider.kind.record(row));
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            await observe(
                object,
                row,
                reconciliation,
                {},
                {
                    ready: unready("DestroyFailed", message),
                },
            );
            throw error;
        }
    }

    // finish the record's deletion
    await reconciliation.execute("finalize", [row]);

    return undefined;
}

/** Read one consumer of a resource from its consumer relationships, absent while nothing consumes it. */
async function consumerOf(
    object: ProvisionedObject<"space">,
    row: ProvisionedRecord,
    database: DatabaseConnection,
): Promise<string | undefined> {
    const [related] = await database
        .select({ subjectId: accessRelationship.subjectId })
        .from(accessRelationship)
        .where(
            and(
                eq(accessRelationship.packageId, object.policy.definition.packageId),
                eq(accessRelationship.type, object.name),
                eq(accessRelationship.objectId, row.id),
                eq(accessRelationship.relation, CONSUMER),
            ),
        )
        .orderBy(accessRelationship.subjectId)
        .limit(1);

    return related?.subjectId;
}

/** Observe provider fields and conditions for the current generation and return the observed row. */
async function observe(
    object: ProvisionedObject<"space">,
    row: ProvisionedRecord,
    reconciliation: ObjectReconciliation<ProvisionedObject<"space">>,
    fields: Partial<Record<keyof ProvisionedRecord, unknown>>,
    conditions: Readonly<Record<string, Condition>>,
): Promise<ProvisionedRecord> {
    // skip an observation the row has
    const columns: Readonly<Record<string, unknown>> = row;
    const isObserved =
        row.observedGeneration === row.generation &&
        Object.entries(conditions).every(([name, condition]) => {
            const current = row.conditions[name];

            return (
                current?.status === condition.status &&
                current.reason === condition.reason &&
                current.message === condition.message
            );
        }) &&
        Object.entries(fields).every(
            ([name, value]) => canonicalize(columns[name] ?? null) === canonicalize(value ?? null),
        );
    if (isObserved) {
        return row;
    }

    // observe the generation, the conditions and the fields as the system
    const [observed] = await reconciliation.server.executeAsSystem(
        object,
        "observe",
        [SystemCall.of(row, { observedGeneration: row.generation, conditions, fields })],
        Date.now(),
    );

    return present(observed, `the observation of resource ${row.id}`);
}

/** Describe a condition that does not hold. */
function unready(reason: string, message: string): Condition {
    return { status: "false", reason, message };
}

/** Keep the reviewed fields of a plan step. */
function planStep(step: Step): Step {
    const { action, target, risk, detail, fields } = step;

    return { action, target, risk, detail, ...(fields === undefined ? {} : { fields }) };
}
