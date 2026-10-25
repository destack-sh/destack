import { accessRelationship } from "@destack/access";
import { and, type DatabaseConnection, eq, Snapshot, Table } from "@destack/db";
import {
    Address,
    type KindState,
    Plan,
    type Provider,
    type ResourceKind,
    type Risk,
    type Step,
} from "@destack/resource";
import { ResourceError } from "@destack/resource/error";
import { nth, Digest, Duration, type Identifier, present, zip } from "@destack/schema";
import { Replica, Scope } from "@destack/sync";
import {
    ObjectWatch,
    type ObjectController,
    type ObjectReconciliation,
} from "../object/controller.ts";
import { type Condition, Observation, type ObservationReport } from "../object/condition.ts";
import { type ObjectType } from "../object/object.ts";
import { CONSUMER } from "../trait/bindable.ts";
import { type ResourceType, type ResourceRow } from "../trait/provisioned.ts";
import { DesiredState } from "../trait/desired.ts";

/** The condition of a resource waiting for draining consumers to stop before recreation. */
export const DRAINING = "Draining";

/** The parsed desired states a controller stores: the platform's, its predecessor's and apps'. */
const DESIRED_STATES_KEPT = 16;

/** What a process reconciling the resources it serves supplies: its providers and its machine. */
export interface ResourceControllerOptions {
    /** The providers the process has, in preference order within each kind. */
    readonly providers: readonly Pick<
        Provider<ResourceKind, ObjectType>,
        "kind" | "code" | "object" | "provision" | "reconcile" | "fence" | "snapshot"
    >[];
    /** The machine running the resources its providers provision. */
    readonly machine: Identifier<"machine">;
}

/** What one reconcile of a resource uses: the process's providers and its desired states. */
interface ReconcileOptions extends ResourceControllerOptions {
    /** Read the desired states of some digests a resource lists. */
    readonly states: (
        row: ResourceRow,
        digests: readonly Digest[],
    ) => Promise<KindState<ResourceKind>[]>;
}

/** The control loop of one kind's provisioned objects through a process's providers. */
export class ResourceController implements ObjectController<ResourceType<"space">> {
    /** The rows the loop takes: every resource of the kind. */
    readonly pending = {};
    /** The consumer relationships, whose changes select the resource they relate. */
    readonly watches: readonly ObjectWatch[];
    /** The provisioned object type. */
    readonly #object: ResourceType<"space">;
    /** The process's providers of the kind and its machine. */
    readonly #options: ResourceControllerOptions;
    /** The resources' kind. */
    readonly #kind: ResourceKind;
    /** The desired states read so far by digest, parsed by the kind, the latest read last. */
    readonly #states = new Map<Digest, unknown>();

    /** Reconcile a kind's provisioned objects via its providers, refusing self-controlled ones. */
    constructor(object: ResourceType<"space">, options: ResourceControllerOptions) {
        // refuse resources another controller reconciles
        if (object.controller !== undefined) {
            throw new TypeError(
                `resources ${object.name} cannot have a controller: their kind provides it`,
            );
        }

        // take the providers of the object's kind
        const providers = options.providers.filter((provider) => provider.object.same(object));
        if (providers.length === 0) {
            throw new TypeError(`no provider here provides ${object.name} resources`);
        }
        this.#object = object;
        this.#kind = present(object.provisioned, `the kind of ${object.name}`).kind;
        this.#options = { ...options, providers };

        // wake a resource once its consumers change, and a scope's resources once the scope comes or goes
        const { packageId } = object.policy.definition;
        this.watches = [
            ObjectWatch.of(accessRelationship, (row) =>
                row.objectPackageId === packageId &&
                row.objectType === object.name &&
                row.relation === CONSUMER
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
    reconcile(reconciliation: ObjectReconciliation<ResourceType<"space">>) {
        return ResourceController.#reconcile(
            this.#object,
            this.#kind,
            nth(reconciliation.rows, 0),
            reconciliation,
            {
                ...this.#options,
                states: (row, digests) => this.#read(reconciliation.database, row, digests),
            },
        );
    }

    /** Read the desired states of some digests a resource lists, each parsed once by the kind. */
    async #read(
        database: DatabaseConnection,
        row: ResourceRow,
        digests: readonly Digest[],
    ): Promise<KindState<ResourceKind>[]> {
        // read and parse the states not stored
        const missing = digests.filter((digest) => !this.#states.has(digest));
        if (missing.length > 0) {
            const parsed = this.#kind.states(
                await DesiredState.read(Snapshot.live(database), row.scope, missing),
            );
            for (const [digest, state] of zip(missing, parsed)) {
                this.#states.set(digest, state);
            }
        }

        // store the latest read states
        const states = digests.map((digest) => present(this.#states.get(digest), digest));
        for (const [digest, state] of zip(digests, states)) {
            this.#states.delete(digest);
            this.#states.set(digest, state);
        }
        for (const digest of [...this.#states.keys()].slice(0, -DESIRED_STATES_KEPT)) {
            this.#states.delete(digest);
        }

        return states;
    }

    /** Select the provider a resource requests, or the first of its kind. */
    static select<Selected extends Pick<ResourceControllerOptions["providers"][number], "code">>(
        providers: readonly Selected[],
        row: Pick<ResourceRow, "provider" | "placement">,
    ): Selected | undefined {
        const requested = row.provider ?? row.placement?.provider ?? null;

        return requested === null
            ? providers[0]
            : providers.find((provider) => provider.code === requested);
    }

    /** Reconcile one resource: provision, retire or apply, returning when to look again. */
    static async #reconcile(
        object: ResourceType<"space">,
        kind: ResourceKind,
        row: ResourceRow,
        reconciliation: ObjectReconciliation<ResourceType<"space">>,
        options: ReconcileOptions,
    ): Promise<number | undefined> {
        // act only on a resource with a local scope and an approval threshold set by its owner
        const { approval } = row;
        if (
            approval === null ||
            !(await ResourceController.#isHeld(reconciliation.database, row.scope))
        ) {
            return undefined;
        }

        // retire a deleted resource
        if (row.deletedAt !== null) {
            return ResourceController.#retire(object, row, row.deletedAt, reconciliation, options);
        }
        // observe an external resource at its identifier below the machine's egress
        else if (kind.management === "external") {
            await ResourceController.#observe(
                object,
                row,
                reconciliation,
                { reference: row.id },
                {
                    Ready: {
                        status: "true",
                        reason: "External",
                        message:
                            row.connection === null
                                ? "called with the connection each call names"
                                : `called with connection ${row.connection}`,
                    },
                },
            );
        }
        // provision a resource whose specification changed and apply its desired states
        else if (!Observation.isCurrent(row, "Ready") || row.reference === null) {
            const provisioned = await ResourceController.#provision(
                object,
                row,
                reconciliation,
                options,
            );
            if (provisioned) {
                await ResourceController.#apply(
                    object,
                    provisioned,
                    approval,
                    reconciliation,
                    options,
                );
            }
        }
        // apply the desired states of a provisioned resource
        else {
            await ResourceController.#apply(object, row, approval, reconciliation, options);
        }

        return undefined;
    }

    /** Decide whether a database has a scope that no transfer fences or replicates here. */
    static async #isHeld(database: DatabaseConnection, scope: string): Promise<boolean> {
        const [link] = await Scope.chain(Snapshot.live(database), scope);

        return (
            link !== undefined &&
            link.movedTo === undefined &&
            !(await Replica.isReplicated(Snapshot.live(database), scope))
        );
    }

    /** Provision a resource through its provider, recording the outcome and returning the row. */
    static async #provision(
        object: ResourceType<"space">,
        row: ResourceRow,
        reconciliation: ObjectReconciliation<ResourceType<"space">>,
        options: ResourceControllerOptions,
    ): Promise<ResourceRow | undefined> {
        // select the requested provider or the first one providing the kind
        const provider = ResourceController.select(options.providers, row);
        if (provider === undefined) {
            const message = `no provider for ${object.name} ${row.name}`;
            await ResourceController.#observe(
                object,
                row,
                reconciliation,
                {},
                { Ready: ResourceController.#unready("NoProvider", message) },
            );

            return undefined;
        }
        // refuse a provider provisioning nothing
        else if (provider.provision === undefined) {
            const message = `provider ${provider.code} of ${object.name} provisions nothing`;
            await ResourceController.#observe(
                object,
                row,
                reconciliation,
                {},
                {
                    Ready: ResourceController.#unready("NoProvisioning", message),
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

            return await ResourceController.#observe(object, row, reconciliation, fields, {
                Ready: ResourceController.#unready("Provisioned", provisioned.reference),
            });
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            await ResourceController.#observe(
                object,
                row,
                reconciliation,
                {},
                {
                    Ready: ResourceController.#unready("ProvisioningFailed", message),
                },
            );
            throw error;
        }
    }

    /** Plan a resource toward its desired states, applying what its approvals allow. */
    static async #apply(
        object: ResourceType<"space">,
        row: ResourceRow,
        approval: Risk,
        reconciliation: ObjectReconciliation<ResourceType<"space">>,
        options: ReconcileOptions,
    ): Promise<void> {
        // require the resource's provider
        const provider = ResourceController.select(options.providers, row);
        if (provider === undefined) {
            const message = `no provider for ${object.name} ${row.name}`;
            await ResourceController.#observe(
                object,
                row,
                reconciliation,
                {},
                { Ready: ResourceController.#unready("NoProvider", message) },
            );

            return;
        }

        // skip resources whose desired states applied already
        const desired = [...row.states, ...row.drainingStates];
        const stateDigest = await Digest.json(desired, reconciliation.store.environment.crypto);
        const appliedDigest = await Digest.json(
            row.appliedStates,
            reconciliation.store.environment.crypto,
        );
        if (appliedDigest === stateDigest && row.conditions["Ready"]?.reason === "Applied") {
            return;
        }

        // plan the desired states or a recreation stopping the draining consumers first
        const planned = await ResourceController.#planRecreation(
            object,
            row,
            provider,
            reconciliation,
            options,
        );
        if (planned === undefined) {
            return;
        }
        const { plan, isDraining } = planned;
        const planDigest = await Plan.digest(plan, reconciliation.store.environment.crypto);
        const waiting = { steps: plan.steps.map((step) => ResourceController.#planStep(step)) };

        // wait for an approval of this exact plan when its risk is at the approval threshold
        if (Plan.isAtLeast(plan, approval) && row.approvedPlan !== planDigest) {
            const message = `approve plan ${planDigest} with ${plan.steps.length} steps`;
            await ResourceController.#observe(
                object,
                row,
                reconciliation,
                { plan: waiting },
                {
                    Ready: ResourceController.#unready("AwaitingApproval", message),
                },
            );
        }
        // report draining for the consumers' owner to stop them
        else if (isDraining) {
            const message = "stopping the draining consumers before applying the plan";
            await ResourceController.#observe(
                object,
                row,
                reconciliation,
                { plan: waiting, approvedPlan: null },
                {
                    Ready: ResourceController.#unready("Recreating", message),
                    [DRAINING]: { status: "true", reason: "Recreating", message },
                },
            );
        }
        // apply the plan and record the desired states it applied
        else {
            const digests = { plan: planDigest, states: desired };
            await ResourceController.#applyPlan(
                object,
                row,
                provider,
                plan,
                digests,
                reconciliation,
                options,
            );
        }
    }

    /** Plan a resource's desired states, or a recreation without draining-only states. */
    static async #planRecreation(
        object: ResourceType<"space">,
        row: ResourceRow,
        provider: ResourceControllerOptions["providers"][number],
        reconciliation: ObjectReconciliation<ResourceType<"space">>,
        options: ReconcileOptions,
    ): Promise<{ readonly plan: Plan; readonly isDraining: boolean } | undefined> {
        // plan the desired states
        const planDigests = async (digests: readonly Digest[]) =>
            provider.reconcile !== undefined
                ? provider.reconcile.plan(
                      provider.kind.record(row),
                      await options.states(row, digests),
                  )
                : { steps: [] };
        try {
            return {
                plan: await planDigests([...row.states, ...row.drainingStates]),
                isDraining: false,
            };
        } catch (error) {
            // plan without the states only draining consumers need
            const recreated =
                ResourceError.is(error, "PLAN_UNREACHABLE") && row.drainingStates.length > 0
                    ? await planDigests(row.states).catch((failure: unknown) => {
                          // block when the active consumers conflict by themselves too
                          if (ResourceError.is(failure, "PLAN_UNREACHABLE")) {
                              return undefined;
                          }
                          throw failure;
                      })
                    : undefined;

            // block on what the declarations must fix and retry another failure
            if (recreated === undefined) {
                const isBlocked = ResourceError.is(error, "PLAN_UNREACHABLE");
                const message = error instanceof Error ? error.message : String(error);
                const reason = isBlocked ? "Blocked" : "PlanFailed";
                await ResourceController.#observe(
                    object,
                    row,
                    reconciliation,
                    {},
                    { Ready: ResourceController.#unready(reason, message) },
                );
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
    static async #applyPlan(
        object: ResourceType<"space">,
        row: ResourceRow,
        provider: ResourceControllerOptions["providers"][number],
        plan: Plan,
        digests: { readonly plan: Digest; readonly states: readonly Digest[] },
        reconciliation: ObjectReconciliation<ResourceType<"space">>,
        options: ReconcileOptions,
    ): Promise<void> {
        try {
            // apply the plan
            if (provider.reconcile !== undefined) {
                await provider.reconcile.apply(
                    provider.kind.record(row),
                    await options.states(row, [...row.states, ...row.drainingStates]),
                    digests.plan,
                );
            }

            // record the desired states it applied and the end of a drain
            const message = `${plan.steps.length} steps applied`;
            await ResourceController.#observe(
                object,
                row,
                reconciliation,
                { appliedStates: [...digests.states], plan: null, approvedPlan: null },
                {
                    Ready: { status: "true", reason: "Applied", message },
                    ...(row.conditions[DRAINING] === undefined
                        ? {}
                        : { [DRAINING]: { status: "false", reason: "Applied", message } }),
                },
            );
        } catch (error) {
            // record the failure, leaving the plan
            const message = error instanceof Error ? error.message : String(error);
            const waiting = {
                steps: plan.steps.map((step) => ResourceController.#planStep(step)),
            };
            await ResourceController.#observe(
                object,
                row,
                reconciliation,
                { plan: waiting },
                {
                    Ready: ResourceController.#unready("ApplyFailed", message),
                },
            );
            throw error;
        }
    }

    /** Remove an unused resource, destroying its content and returning when its retention ends. */
    static async #retire(
        object: ResourceType<"space">,
        row: ResourceRow,
        requestedAt: number,
        reconciliation: ObjectReconciliation<ResourceType<"space">>,
        options: ResourceControllerOptions,
    ): Promise<number | undefined> {
        // retain the record and content of retained resources until claimed again or expired
        const retention = row.retention;
        const ends =
            typeof retention === "object"
                ? requestedAt + Duration.milliseconds(retention.within)
                : undefined;
        if (retention === "forever" || (ends !== undefined && ends > reconciliation.now)) {
            if (row.conditions["Ready"]?.reason !== "Retained") {
                const message = "content retained after removal";
                await ResourceController.#observe(
                    object,
                    row,
                    reconciliation,
                    {},
                    {
                        Ready: ResourceController.#unready("Retained", message),
                    },
                );
            }

            return ends === undefined ? undefined : ends - reconciliation.now;
        }

        // wait until no consumer uses the resource
        const consumer = await ResourceController.#consumerOf(object, row, reconciliation.database);
        if (consumer !== undefined) {
            const message = `consumed by ${consumer}`;
            await ResourceController.#observe(
                object,
                row,
                reconciliation,
                {},
                { Ready: ResourceController.#unready("InUse", message) },
            );

            return undefined;
        }

        // destroy provisioned content and record a failure before retrying it
        const provider = ResourceController.select(options.providers, row);
        if (row.reference !== null && provider?.provision !== undefined) {
            try {
                await provider.provision.destroy(provider.kind.record(row));
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                await ResourceController.#observe(
                    object,
                    row,
                    reconciliation,
                    {},
                    {
                        Ready: ResourceController.#unready("DestroyFailed", message),
                    },
                );
                throw error;
            }
        }

        // finish the record's deletion
        await reconciliation.mutate([object.call("finalize", object.target(row))]);

        return undefined;
    }

    /** Read one consumer of a resource from its consumer relationships, if any. */
    static async #consumerOf(
        object: ResourceType<"space">,
        row: ResourceRow,
        database: DatabaseConnection,
    ): Promise<string | undefined> {
        const [related] = await database
            .select({ subjectId: accessRelationship.subjectId })
            .from(accessRelationship)
            .where(
                and(
                    eq(accessRelationship.objectPackageId, object.policy.definition.packageId),
                    eq(accessRelationship.objectType, object.name),
                    eq(accessRelationship.objectId, row.id),
                    eq(accessRelationship.relation, CONSUMER),
                ),
            )
            .orderBy(accessRelationship.subjectId)
            .limit(1);

        return related?.subjectId;
    }

    /** Observe provider fields and conditions for the current generation, returning the row. */
    static async #observe(
        object: ResourceType<"space">,
        row: ResourceRow,
        reconciliation: ObjectReconciliation<ResourceType<"space">>,
        fields: NonNullable<ObservationReport["fields"]>,
        conditions: Readonly<Record<string, Condition>>,
    ): Promise<ResourceRow> {
        // observe the generation, the conditions and the fields once
        const observed = await reconciliation.observe(object, row, { conditions, fields });

        return Table.selectSchema(object.table).parse(observed);
    }

    /** Describe an unmet condition. */
    static #unready(reason: string, message: string): Condition {
        return { status: "false", reason, message };
    }

    /** Take the reviewed fields of a plan step. */
    static #planStep(step: Step): Step {
        const { action, target, risk, detail, fields } = step;

        return { action, target, risk, detail, ...(fields === undefined ? {} : { fields }) };
    }
}
