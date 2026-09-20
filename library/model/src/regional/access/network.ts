import {
    check,
    type Column,
    foreignKey,
    identifier,
    integer,
    json,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
    uniqueIndex,
} from "@destack/db";
import { NetworkPolicyDefinition } from "@destack/space";
import { provenanceChecks, provenanceColumns } from "../../source/index.ts";
import { RecordSource } from "../../source/source.ts";
import { space } from "../space/space.ts";
import { installation } from "../space/installation.ts";

/** Outbound network restrictions administered in this region. */
export const networkPolicy = table("network_policy", {
    ...recordColumns("network-policy"),
    ...provenanceColumns(),
    /** The account administering the policy, resolved through the global directory. */
    accountId: identifier("account_id", "account").notNull(),
    /** The scope to which every revision applies. */
    scope: text("scope", { enum: ["account", "space", "installation", "workload"] }).notNull(),
    /** The restricted space; absent for account restrictions. */
    spaceId: identifier("space_id", "space"),
    /** The restricted installation; absent for account and space policies. */
    installationId: identifier("installation_id", "installation"),
    /** The package-local workload; present only for workload policies. */
    workload: text("workload"),
    /** The desired revision generation. */
    generation: integer("generation").notNull().default(1),
    /** The active immutable revision, absent before initial activation. */
    currentRevisionId: identifier("current_revision_id", "network-policy-revision"),
}, (policy) => [
    ...provenanceChecks("network_policy", policy),
    foreignKey({
        columns: [policy.accountId, policy.spaceId],
        foreignColumns: [space.accountId, space.id],
    }).onDelete("restrict"),
    foreignKey({
        columns: [policy.id, policy.currentRevisionId],
        foreignColumns: [networkPolicyRevision.policyId, networkPolicyRevision.id],
    }).onDelete("restrict"),
    foreignKey({
        columns: [policy.spaceId, policy.installationId],
        foreignColumns: [installation.spaceId, installation.id],
    }).onDelete("restrict"),
    uniqueIndex("network_policy_installation").on(policy.installationId)
        .where(sql`${policy.scope} = 'installation'`),
    uniqueIndex("network_policy_workload").on(policy.installationId, policy.workload)
        .where(sql`${policy.scope} = 'workload'`),
    uniqueIndex("network_policy_account").on(policy.accountId)
        .where(sql`${policy.scope} = 'account'`),
    uniqueIndex("network_policy_space").on(policy.spaceId)
        .where(sql`${policy.scope} = 'space'`),
    check(
        "network_policy_scope",
        sql`
        (${policy.scope} = 'account' AND ${policy.spaceId} IS NULL AND ${policy.installationId} IS NULL AND ${policy.workload} IS NULL) OR
        (${policy.scope} = 'space' AND ${policy.spaceId} IS NOT NULL AND ${policy.installationId} IS NULL AND ${policy.workload} IS NULL) OR
        (${policy.scope} = 'installation' AND ${policy.spaceId} IS NOT NULL AND ${policy.installationId} IS NOT NULL AND ${policy.workload} IS NULL) OR
        (${policy.scope} = 'workload' AND ${policy.spaceId} IS NOT NULL AND ${policy.installationId} IS NOT NULL AND ${policy.workload} IS NOT NULL AND length(${policy.workload}) > 0)
    `,
    ),
    check("network_policy_generation", sql`${policy.generation} > 0`),
]);

/** An immutable network policy definition retained for evaluation and audit. */
export const networkPolicyRevision = table("network_policy_revision", {
    ...recordColumns("network-policy-revision"),
    /** The policy whose scope applies to this definition. */
    policyId: identifier("policy_id", "network-policy").notNull()
        .references((): Column => networkPolicy.id, { onDelete: "restrict" }),
    /** The policy generation that produced this revision. */
    generation: integer("generation").notNull(),
    /** The complete outbound network rules. */
    definition: json("definition", NetworkPolicyDefinition).notNull(),
    /** The exact source declaration that produced this revision, if source-managed. */
    source: json("source", RecordSource),
    /** SHA-256 of the canonical policy definition. */
    digest: text("digest").notNull(),
}, (revision) => [
    unique("network_policy_revision_policy_id").on(revision.policyId, revision.id),
    unique("network_policy_revision_generation").on(revision.policyId, revision.generation),
    check("network_policy_revision_generation_positive", sql`${revision.generation} > 0`),
    check(
        "network_policy_revision_digest",
        sql`
        length(${revision.digest}) = 64 AND ${revision.digest} NOT GLOB '*[^a-f0-9]*'
    `,
    ),
]);

/** A persisted account, space, installation, or workload network policy. */
export type NetworkPolicy = Select<typeof networkPolicy>;
/** An immutable policy revision. */
export type NetworkPolicyRevision = Select<typeof networkPolicyRevision>;
