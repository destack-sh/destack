import { Manager } from "@destack/object";
import { type DatabaseConnection, eq, type Table } from "@destack/db";
import type { Stack } from "@destack/object/server";
import { identifier, schema } from "@destack/schema";
import { digest } from "@destack/schema/json";
import { SpaceInstallation } from "../declare/installation.ts";
import * as base from "../object/index.ts";
import { installation, networkPolicyVersion, packagePolicyVersion } from "../object/index.ts";
import { type NetworkPolicyDefinition, SpacePolicy } from "../policy/index.ts";
import type { PackagePolicyDefinition } from "../policy/package.ts";

/** A network policy a stack declares for its space, an installation or one of its workloads. */
interface NetworkPolicyDeclaration {
    /** The installation's declaration name, absent for the whole space. */
    readonly installation?: string;
    /** The package-local workload restricted, present only with an installation. */
    readonly workload?: string;
    /** The complete outbound network rules. */
    readonly definition: NetworkPolicyDefinition;
}

/** Network policies a stack declares for its space, installations and workloads, keyed by path. */
export const networkPolicy = base.networkPolicy.declare({
    after: [installation],
    keys: ["policies", "installations"],
    collect: (document) => {
        // restrict the whole space
        const policies: Record<string, NetworkPolicyDeclaration> = {};
        const space = SpacePolicy.optional().parse(document.policies);
        if (space?.network) {
            policies.network = { definition: space.network };
        }

        // restrict declared installations and their named workloads
        const installations = schema
            .record(schema.string(), SpaceInstallation)
            .parse(document.installations ?? {});
        for (const [name, declared] of Object.entries(installations)) {
            if (declared.policies?.network) {
                policies[`installations/${name}`] = {
                    installation: name,
                    definition: declared.policies.network,
                };
            }
            for (const [workload, restriction] of Object.entries(
                declared.policies?.workloads ?? {},
            )) {
                policies[`installations/${name}/${workload}`] = {
                    installation: name,
                    workload,
                    definition: restriction.network,
                };
            }
        }

        return policies;
    },
    resolve: async (_name, declared: NetworkPolicyDeclaration, stack) => ({
        installationId:
            declared.installation === undefined
                ? undefined
                : identifier("installation").parse(
                      await stack.require(installation, declared.installation),
                  ),
        workload: declared.workload,
        definition: declared.definition,
    }),
    values: (_name, resolved) => ({
        level: resolved.workload
            ? ("workload" as const)
            : resolved.installationId
              ? ("installation" as const)
              : ("space" as const),
        installationId: resolved.installationId ?? null,
        workload: resolved.workload ?? null,
    }),
    changed: async (database, row, resolved) =>
        (await currentDigest(database, base.networkPolicy, networkPolicyVersion, row.id)) !==
        (await digest(resolved.definition)),
    written: async (stack, row, resolved) => {
        // retain the declared definition as the policy's next immutable version
        await addVersion(stack, base.networkPolicy, networkPolicyVersion, row, {
            definition: resolved.definition,
            source: Manager.read(row),
            digest: await digest(resolved.definition),
        });
    },
    retire: {
        isRetiring: async (database, row) =>
            (await currentDigest(database, base.networkPolicy, networkPolicyVersion, row.id)) ===
            undefined,
        start: async (stack, row) => {
            await stack.invoke(base.networkPolicy, "point", {
                id: row.id,
                currentVersionId: null,
            });
        },
    },
});

/** The package admission policy a stack declares for its space. */
export const packagePolicy = base.packagePolicy.declare({
    keys: ["policies"],
    collect: (document): Record<string, PackagePolicyDefinition> => {
        const space = SpacePolicy.optional().parse(document.policies);

        return space?.packages ? { packages: space.packages } : {};
    },
    values: () => ({ level: "space" as const }),
    changed: async (database, row, resolved) =>
        (await currentDigest(database, base.packagePolicy, packagePolicyVersion, row.id)) !==
        (await digest(resolved)),
    written: async (stack, row, resolved) => {
        // retain the declared rules as the policy's next immutable version
        await addVersion(stack, base.packagePolicy, packagePolicyVersion, row, {
            definition: resolved,
            source: Manager.read(row),
            digest: await digest(resolved),
        });
    },
    retire: {
        isRetiring: async (database, row) =>
            (await currentDigest(database, base.packagePolicy, packagePolicyVersion, row.id)) ===
            undefined,
        start: async (stack, row) => {
            await stack.invoke(base.packagePolicy, "point", {
                id: row.id,
                currentVersionId: null,
            });
        },
    },
});

/** Retain a definition as a policy's next version and point the policy at it. */
async function addVersion(
    stack: Stack,
    owner: typeof base.networkPolicy | typeof base.packagePolicy,
    version: typeof networkPolicyVersion | typeof packagePolicyVersion,
    policy: { readonly id: string },
    values: { readonly definition: unknown; readonly source: unknown; readonly digest: string },
): Promise<void> {
    const created = (await stack.invoke(version, "create", {
        parentId: policy.id,
        ...values,
    })) as { readonly id: string };
    await stack.invoke(owner, "point", { id: policy.id, currentVersionId: created.id });
}

/** Read the digest of a policy's version in force, absent while it has none. */
async function currentDigest(
    database: DatabaseConnection,
    owner: typeof base.networkPolicy | typeof base.packagePolicy,
    version: typeof networkPolicyVersion | typeof packagePolicyVersion,
    policyId: string,
): Promise<string | undefined> {
    // read the version the policy points at
    const owned: Table = owner.table;
    const policy = owned as Table & Record<string, never>;
    const declared: Table = version.table;
    const table = declared as Table & Record<string, never>;
    const [current] = await database
        .select({ digest: table.digest })
        .from(policy)
        .innerJoin(table, eq(table.id, policy.currentVersionId))
        .where(eq(policy.id, policyId));

    return current === undefined ? undefined : schema.string().parse(current.digest);
}
