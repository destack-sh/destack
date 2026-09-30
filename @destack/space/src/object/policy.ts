import { through } from "@destack/access";
import { check, foreignKey, sql, uniqueIndex } from "@destack/db";
import { defineObject, field, Manager, method, type ObjectType } from "@destack/object";
import { schema } from "@destack/schema";
import { Digest } from "@destack/package/file";
import { NetworkPolicyDefinition } from "../policy/network.ts";
import { PackagePolicyDefinition } from "../policy/package.ts";
import { installation } from "./installation.ts";
import { account } from "@destack/account/object";
import { space } from "./space.ts";

/** Point a policy at the version in force, or at none, as its stack does. */
const point = method({
    permission: null,
    isSystem: true,
    input: schema.object({
        /** The version in force, null for none. */
        currentVersionId: schema.string().nullable(),
    }),
}).handle((call) => call.observe({ currentVersionId: call.input.currentVersionId }));

/** Outbound network rules for a space, an installation or one of its workloads. */
export const networkPolicy = defineObject({
    name: "network-policy",
    plural: "networkPolicies",
    scope: [account, space],
    declarable: { schema: NetworkPolicyDefinition },
    fields: {
        /** What the policy restricts: its account, its space, one installation of the space, or one workload of it. */
        level: field.enum(["account", "space", "installation", "workload"]),
        /** The restricted installation, for installation and workload policies. */
        installationId: field.reference<"installation">((): ObjectType => installation).optional(),
        /** The package-local workload, for workload policies. */
        workload: field.string().optional(),
        /** The desired generation, the number of the version it makes current. */
        generation: field.integer().default(1),
        /** The version in force, absent while the policy has none. */
        currentVersionId: field
            .reference<"network-policy-version">((): ObjectType => networkPolicyVersion, {
                delete: "null",
            })
            .optional(),
    },
    constraints: (policy) => [
        foreignKey({
            columns: [policy.scope, policy.managerInstallationId],
            foreignColumns: [installation.table.scope, installation.table.id],
        }),
        foreignKey({
            columns: [policy.scope, policy.installationId],
            foreignColumns: [installation.table.scope, installation.table.id],
        }).onDelete("restrict"),
        uniqueIndex("network_policy_installation")
            .on(policy.installationId)
            .where(sql`${policy.level} = 'installation'`),
        uniqueIndex("network_policy_workload")
            .on(policy.installationId, policy.workload)
            .where(sql`${policy.level} = 'workload'`),
        uniqueIndex("network_policy_scope")
            .on(policy.scope)
            .where(sql`${policy.level} IN ('account', 'space')`),
        check(
            "network_policy_level",
            sql`
            (${policy.level} IN ('account', 'space') AND ${policy.installationId} IS NULL AND ${policy.workload} IS NULL) OR
            (${policy.level} = 'installation' AND ${policy.installationId} IS NOT NULL AND ${policy.workload} IS NULL) OR
            (${policy.level} = 'workload' AND ${policy.installationId} IS NOT NULL AND ${policy.workload} IS NOT NULL AND length(${policy.workload}) > 0)
        `,
        ),
        check("network_policy_generation", sql`${policy.generation} > 0`),
    ],
    permissions: ["read", "update"],
    methods: { point },
});

/** An immutable version of a network policy. */
export const networkPolicyVersion = defineObject({
    name: "network-policy-version",
    plural: "networkPolicyVersions",
    scope: [account, space],
    nested: { in: networkPolicy, delete: "cascade", receive: "update" },
    versioned: true,
    fields: {
        /** The policy as this version defines it. */
        definition: field.json(NetworkPolicyDefinition),
        /** The stack that declared this version, absent for versions written directly. */
        source: field.json(Manager.schema).optional(),
        /** The SHA-256 digest of the canonical definition. */
        digest: field.string(Digest),
    },
    permissions: { read: through("parent", "read"), update: through("parent", "update") },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, { isSystem: true }),
    },
});

/** The packages a space admits. */
export const packagePolicy = defineObject({
    name: "package-policy",
    plural: "packagePolicies",
    scope: [account, space],
    declarable: { schema: PackagePolicyDefinition },
    fields: {
        /** What the policy restricts: its account or its space. */
        level: field.enum(["account", "space"]),
        /** The desired generation, the number of the version it makes current. */
        generation: field.integer().default(1),
        /** The version in force, absent while the policy has none. */
        currentVersionId: field
            .reference<"package-policy-version">((): ObjectType => packagePolicyVersion, {
                delete: "null",
            })
            .optional(),
    },
    constraints: (policy) => [
        foreignKey({
            columns: [policy.scope, policy.managerInstallationId],
            foreignColumns: [installation.table.scope, installation.table.id],
        }),
        uniqueIndex("package_policy_scope").on(policy.scope),
        check("package_policy_generation", sql`${policy.generation} > 0`),
    ],
    permissions: ["read", "update"],
    methods: { point },
});

/** An immutable version of a package policy. */
export const packagePolicyVersion = defineObject({
    name: "package-policy-version",
    plural: "packagePolicyVersions",
    scope: [account, space],
    nested: { in: packagePolicy, delete: "cascade", receive: "update" },
    versioned: true,
    fields: {
        /** The policy as this version defines it. */
        definition: field.json(PackagePolicyDefinition),
        /** The stack that declared this version, absent for versions written directly. */
        source: field.json(Manager.schema).optional(),
        /** The SHA-256 digest of the canonical definition. */
        digest: field.string(Digest),
    },
    permissions: { read: through("parent", "read"), update: through("parent", "update") },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, { isSystem: true }),
    },
});
