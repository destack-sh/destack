import type { Identifier } from "@destack/schema";
import { and, type DatabaseConnection, eq, type Table } from "@destack/db";
import { Condition } from "@destack/db/query";
import type { ObjectReconciliation, ObjectType } from "@destack/object";
import { SystemCall } from "@destack/object/server";
import {
    Address,
    Plan,
    type Risk,
    type Step,
    type ResourceKind,
    Provider,
} from "@destack/resource";
import { PlanError } from "@destack/resource/error";
import type { ResourceState } from "@destack/package/declare";
import { canonicalize, digest } from "@destack/schema/json";
import { ServiceError } from "@destack/service/error";
import { SpaceError } from "../error/index.ts";
import {
    Binding,
    binding,
    capture,
    Deployment,
    deployment,
    type Resource,
    resource,
    space,
    SpaceCell,
} from "../object/index.ts";

/** A cell's providers by resource kind, in preference order. */
export class ProviderIndex {
    /** The host or region serving the providers' resources. */
    readonly cell: SpaceCell;
    /** The providers by kind, in preference order. */
    readonly byKind: ReadonlyMap<string, readonly Provider<ResourceKind, ObjectType>[]>;

    /** Index a cell's providers by kind, keeping their order. */
    constructor(cell: SpaceCell, providers: readonly Provider<ResourceKind, ObjectType>[]) {
        // group the providers by the kind they provide
        const byKind = new Map<string, Provider<ResourceKind, ObjectType>[]>();
        for (const provider of providers) {
            byKind.set(provider.kind.name, [...(byKind.get(provider.kind.name) ?? []), provider]);
        }

        // keep the cell and its providers
        this.cell = cell;
        this.byKind = byKind;
    }

    /** Select the provider a resource requests, or the first providing its kind. */
    select(row: Resource): Provider<ResourceKind, ObjectType> | undefined {
        const providers = this.byKind.get(row.kind) ?? [];

        return row.providerCode === null && row.requestedProviderCode === null
            ? providers[0]
            : providers.find(
                  (provider) => provider.code === (row.providerCode ?? row.requestedProviderCode),
              );
    }

    /** List the facets the providers create beside their resources, by their names in camel case. */
    facets(): Readonly<Record<string, ObjectType>> {
        const facets = [...this.byKind.values()]
            .flat()
            .flatMap((provider) => (provider.facet === undefined ? [] : [provider.facet]));

        return Object.fromEntries(
            facets.map((facet) => [
                facet.name.replace(/-([a-z])/g, (_, letter: string) => letter.toUpperCase()),
                facet,
            ]),
        );
    }
}

/**
 * Read the desired states a resource's bindings require and refuse a resource without them applied.
 *
 * A plan that waits for approval blocks the transfer.
 */
export async function appliedStates(
    database: DatabaseConnection,
    row: Resource,
): Promise<ResourceState[]> {
    // require the recorded digest of the bindings' desired states, with no plan waiting
    const desired = await desiredStates(database, row);
    const applied = row.status.state;
    if (applied?.digest !== (await digest(desired)) || applied.plan !== undefined) {
        throw new ServiceError("PRECONDITION_FAILED", {
            message: `resource ${row.id} has not applied its bindings' desired states: approve or reject its plan first`,
        });
    }

    return desired;
}

/** Serve resources: apply their declarations, and reconcile each space's resources with its providers. */
export function serveResources(providers: ProviderIndex) {
    return resource
        .declare({
            values: (name, desired) => {
                // TODO #Incomplete: adopt resources from other spaces after ownership and residency checks
                if (desired.adopt) {
                    throw new SpaceError(
                        "NOT_IMPLEMENTED",
                        `resource adoption is not supported: ${name}`,
                    );
                }
                // require the provider connecting a resource whose reference the stack declares
                const { declaration, placement, reference } = desired;
                if (reference !== undefined && placement?.provider === undefined) {
                    throw new SpaceError(
                        "INVALID_DEFINITION",
                        `resource ${name} declares its reference without the provider connecting to it`,
                    );
                }

                return {
                    name,
                    definitionPackageId: declaration.package.id,
                    definitionVersion: declaration.package.version,
                    definitionName: declaration.name,
                    kind: declaration.kind,
                    spec: declaration.spec,
                    retention: desired.retention,
                    requestedProviderCode: placement?.provider ?? null,
                    requestedLocation: placement?.location ?? null,
                    requestedHostId: placement?.host ?? null,
                    ...(reference === undefined
                        ? { origin: "provisioned" as const }
                        : {
                              origin: "declared" as const,
                              reference,
                              providerCode: placement!.provider,
                          }),
                    tags: desired.tags,
                };
            },
        })
        .control({
            pending: Condition.all(),
            watches: [
                // the resource a changed binding targets
                {
                    table: binding.table,
                    keys: (row) =>
                        String(row.target).startsWith(`${resource.identity}-`)
                            ? [{ id: row.target }]
                            : [],
                },
                // the resources a changed deployment captured
                {
                    table: deployment.table,
                    keys: async (row, database) =>
                        database
                            .select({ id: capture.table.target })
                            .from(capture.table)
                            .where(
                                and(
                                    eq(capture.table.scope, row.scope as Identifier<"space">),
                                    eq(
                                        capture.table.deploymentId,
                                        row.id as Identifier<"deployment">,
                                    ),
                                ),
                            ),
                },
                // the resources of a space whose approval threshold or service changed
                {
                    table: space.table,
                    keys: async (row, database) =>
                        database
                            .select({ id: resource.table.id })
                            .from(resource.table)
                            .where(eq(resource.table.scope, row.id as Identifier<"space">)),
                },
            ],
            reconcile: (reconciliation) =>
                reconcile(reconciliation.rows[0]!, reconciliation, providers),
        });
}

/** Reconcile one resource: provision, retire, and apply the desired states of its bindings. */
async function reconcile(
    row: Resource,
    reconciliation: ObjectReconciliation,
    providers: ProviderIndex,
): Promise<undefined> {
    // act only while this cell serves the space, at its approval threshold
    const [served] = await reconciliation.database
        .select({ approval: space.table.approval })
        .from(space.table)
        .where(and(eq(space.table.id, row.scope), SpaceCell.served()));
    if (served === undefined) {
        return undefined;
    }

    // retire a resource whose deletion was requested
    if (row.deletionRequestedAt !== null) {
        await retire(row, reconciliation, providers);
    }
    // observe a resource whose stack declares its reference ready as declared
    else if (row.origin === "declared") {
        await observe(row, reconciliation, {}, "true", "Declared", `declared at ${row.reference}`);
    }
    // provision a resource whose declaration changed, then apply its desired states
    else if (row.generation > row.observedGeneration || row.reference === null) {
        const provisioned = await provision(row, reconciliation, providers);
        if (provisioned) {
            await apply(provisioned, served.approval, reconciliation, providers);
        }
    }
    // apply the desired states of a provisioned resource whose bindings changed
    else {
        await apply(row, served.approval, reconciliation, providers);
    }

    return undefined;
}

/** Provision a resource through its provider, recording the outcome and returning the provisioned row. */
async function provision(
    row: Resource,
    reconciliation: ObjectReconciliation,
    providers: ProviderIndex,
): Promise<Resource | undefined> {
    // select the requested provider, or the first one providing the kind
    const provider = providers.select(row);
    if (!provider) {
        await observe(
            row,
            reconciliation,
            {},
            "false",
            "NoProvider",
            `no provider for ${row.kind}`,
        );

        return undefined;
    } else if (!Provider.provisions(provider)) {
        const message = `provider ${provider.code} of ${row.kind} provisions nothing`;
        await observe(row, reconciliation, {}, "false", "NoProvisioning", message);

        return undefined;
    }

    // record the provider reference for this generation, or the failure before retrying it
    try {
        const provisioned = await provider.provision(provider.kind.record(row));
        await createFacet(provider, row, reconciliation);
        const fields = {
            providerCode: provider.code,
            reference: provisioned.reference,
            location: provisioned.location ?? null,
            hostId: SpaceCell.host(providers.cell),
        };

        return await observe(
            row,
            reconciliation,
            fields,
            "false",
            "Provisioned",
            provisioned.reference,
        );
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        await observe(row, reconciliation, {}, "false", "ProvisioningFailed", message);
        throw error;
    }
}

/** Plan a provisioned resource toward its bindings' desired states and apply what policy or approval allows. */
async function apply(
    row: Resource,
    approval: Risk,
    reconciliation: ObjectReconciliation,
    providers: ProviderIndex,
): Promise<void> {
    // require the resource's provider
    const provider = providers.select(row);
    if (!provider) {
        await observe(
            row,
            reconciliation,
            {},
            "false",
            "NoProvider",
            `no provider for ${row.kind}`,
        );

        return;
    }

    // skip resources whose bindings' desired states applied already
    const database = reconciliation.database;
    const desired = await desiredStates(database, row);
    const stateDigest = await digest(desired);
    const applied = row.status.state;
    if (applied?.digest === stateDigest && row.conditions.ready?.reason === "Applied") {
        return;
    }

    // plan the desired states, or a recreation over the draining releases they conflict with
    const planOf = (states: readonly ResourceState[]) =>
        Provider.reconciles(provider)
            ? provider.plan(provider.kind.record(row), provider.kind.states(states))
            : Promise.resolve({ steps: [] });
    let plan: Plan;
    let stopped: readonly Deployment[] = [];
    try {
        plan = await planOf(desired);
    } catch (error) {
        // plan without the draining releases, stopping them first
        const draining = await drainingOf(database, row);
        const recreated =
            error instanceof PlanError && draining.length > 0
                ? await planOf(await desiredStates(database, row, "active")).catch(
                      (failure: unknown) => {
                          // block when the active release alone conflicts too
                          if (failure instanceof PlanError) {
                              return undefined;
                          }
                          throw failure;
                      },
                  )
                : undefined;

        // block on what the declarations must fix, and retry another failure
        if (recreated === undefined) {
            const isBlocked = error instanceof PlanError;
            const message = error instanceof Error ? error.message : String(error);
            await observe(
                row,
                reconciliation,
                {},
                "false",
                isBlocked ? "Blocked" : "PlanFailed",
                message,
            );
            if (!isBlocked) {
                throw error;
            }

            return;
        }
        stopped = draining;
        plan = {
            steps: [
                ...draining.map((entry) => ({
                    action: "delete" as const,
                    target: Address.join("deployment", entry.id),
                    risk: "backward-incompatible" as const,
                    detail: `stop release ${entry.release} before the release replacing it`,
                })),
                ...recreated.steps,
            ],
        };
    }
    const planDigest = await Plan.digest(plan);
    const status = {
        state: { digest: stateDigest, plan: { steps: plan.steps.map(planStep) } },
    };

    // wait for an approval of this exact plan when its risk reaches the space's approval threshold
    if (Plan.reaches(plan, approval) && row.approvedPlan !== planDigest) {
        const message = `approve plan ${planDigest} with ${plan.steps.length} steps`;
        await observe(row, reconciliation, { status }, "false", "AwaitingApproval", message);

        return;
    }

    // stop the draining releases of an approved recreation, applying the rest once they retire
    if (stopped.length > 0) {
        await reconciliation.server.executeAsSystem(
            deployment,
            "update",
            stopped.map((entry) => SystemCall.of(entry, { status: "stopping" })),
            reconciliation.now,
        );
        const message = `stopping ${stopped.length} releases before the release replacing them`;
        await observe(
            row,
            reconciliation,
            { status, approvedPlan: null },
            "false",
            "Recreating",
            message,
        );

        return;
    }

    // apply the plan and record the desired states it applied
    try {
        if (Provider.reconciles(provider)) {
            await provider.apply(
                provider.kind.record(row),
                provider.kind.states(desired),
                planDigest,
            );
        }
        await observe(
            row,
            reconciliation,
            { status: { state: { digest: stateDigest } }, approvedPlan: null },
            "true",
            "Applied",
            `${plan.steps.length} steps applied`,
        );
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        await observe(row, reconciliation, { status }, "false", "ApplyFailed", message);
        throw error;
    }
}

/** Remove a resource once unbound, destroying its content only when retention allows. */
async function retire(
    row: Resource,
    reconciliation: ObjectReconciliation,
    providers: ProviderIndex,
): Promise<void> {
    // keep the record and content of retained resources until they are declared again
    if (row.retention === "retain") {
        if (row.conditions.ready?.reason !== "Retained") {
            await observe(
                row,
                reconciliation,
                {},
                "false",
                "Retained",
                "content retained after removal",
            );
        }

        return;
    }

    // wait until no installation binds the resource and no live deployment runs with it
    const use = await Binding.inUse(reconciliation.database, row);
    if (use !== undefined) {
        await observe(row, reconciliation, {}, "false", "InUse", use);

        return;
    }

    // destroy provisioned content and its facet, retrying a failure after recording it
    const provider = providers.select(row);
    if (row.reference !== null && provider && Provider.provisions(provider)) {
        try {
            await provider.destroy(provider.kind.record(row));
            await deleteFacet(provider, row, reconciliation);
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            await observe(row, reconciliation, {}, "false", "DestroyFailed", message);
            throw error;
        }
    }

    // finish the record's deletion
    await reconciliation.execute("finalize", [row]);
}

/** Create the facet sharing a provisioned resource's identity, once. */
async function createFacet(
    provider: Provider<ResourceKind, ObjectType>,
    row: Resource,
    reconciliation: ObjectReconciliation,
): Promise<void> {
    const facet = provider.facet;
    if (facet !== undefined && (await facetOf(facet, row, reconciliation.database)) === undefined) {
        await reconciliation.server.executeAsSystem(
            facet,
            "create",
            [{ scope: row.scope, id: row.id, input: {} }],
            reconciliation.now,
        );
    }
}

/** Delete the facet of a destroyed resource, once. */
async function deleteFacet(
    provider: Provider<ResourceKind, ObjectType>,
    row: Resource,
    reconciliation: ObjectReconciliation,
): Promise<void> {
    const facet = provider.facet;
    const found =
        facet === undefined ? undefined : await facetOf(facet, row, reconciliation.database);
    if (facet !== undefined && found !== undefined) {
        await reconciliation.server.executeAsSystem(
            facet,
            "delete",
            [SystemCall.of(found)],
            reconciliation.now,
        );
    }
}

/** Read the facet sharing a resource's identity, absent before its creation. */
async function facetOf(
    facet: ObjectType,
    row: Resource,
    database: DatabaseConnection,
): Promise<Record<string, unknown> | undefined> {
    const table = facet.table as Table & Record<string, never>;
    const [found] = (await database
        .select()
        .from(table)
        .where(eq(table.id, row.id as never))) as Record<string, unknown>[];

    return found;
}

/** Observe provider fields and the ready condition for the current generation and return the observed row. */
async function observe(
    row: Resource,
    reconciliation: ObjectReconciliation,
    fields: Partial<typeof resource.table.$inferInsert>,
    status: "true" | "false",
    reason: string,
    message: string,
): Promise<Resource> {
    // skip an observation the row has
    const ready = row.conditions.ready;
    const isObserved =
        row.observedGeneration === row.generation &&
        ready?.status === status &&
        ready.reason === reason &&
        ready.message === message &&
        Object.entries(fields).every(
            ([name, value]) =>
                canonicalize(row[name as keyof Resource] ?? null) === canonicalize(value ?? null),
        );
    if (isObserved) {
        return row;
    }

    // observe the generation, the ready condition and the fields as the system
    const [observed] = await reconciliation.server.executeAsSystem(
        resource,
        "observe",
        [
            {
                ...SystemCall.of(row),
                input: {
                    observedGeneration: row.generation,
                    conditions: { ready: { status, reason, message } },
                    fields,
                },
            },
        ],
        Date.now(),
    );

    return observed as Resource;
}

/** Read the desired states of a resource: its bindings' current ones and those its serving or active deployments captured, once each. */
async function desiredStates(
    database: DatabaseConnection,
    row: Pick<Resource, "id" | "scope">,
    deployments: "serving" | "active" = "serving",
): Promise<ResourceState[]> {
    // read the bindings' states and the states the chosen deployments run with, through their space's target indexes
    const bound = await database
        .select({ state: binding.table.state })
        .from(binding.table)
        .where(and(eq(binding.table.scope, row.scope), eq(binding.table.target, row.id)))
        .orderBy(binding.table.id);
    const captured = await database
        .select({ state: capture.table.state })
        .from(capture.table)
        .innerJoin(deployment.table, eq(deployment.table.id, capture.table.deploymentId))
        .where(
            and(
                eq(capture.table.scope, row.scope),
                eq(capture.table.target, row.id),
                deployments === "serving"
                    ? Deployment.serving()
                    : eq(deployment.table.status, "active"),
            ),
        )
        .orderBy(capture.table.id);

    // keep each distinct state once
    const states = new Map(
        [...bound, ...captured].map((entry) => [canonicalize(entry.state), entry.state]),
    );

    return [...states.values()];
}

/** Read the draining deployments that captured a resource. */
async function drainingOf(
    database: DatabaseConnection,
    row: Pick<Resource, "id" | "scope">,
): Promise<Deployment[]> {
    const rows = await database
        .selectDistinct({ deployment: deployment.table })
        .from(capture.table)
        .innerJoin(deployment.table, eq(deployment.table.id, capture.table.deploymentId))
        .where(
            and(
                eq(capture.table.scope, row.scope),
                eq(capture.table.target, row.id),
                eq(deployment.table.status, "draining"),
            ),
        );

    return rows.map((entry) => entry.deployment);
}

/** Keep the reviewed fields of a plan step. */
function planStep(step: Step): Step {
    const { action, target, risk, detail, fields } = step;

    return { action, target, risk, detail, ...(fields === undefined ? {} : { fields }) };
}
