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
import { PackagePolicyDefinition } from "@destack/space";
import { provenanceChecks, provenanceColumns } from "../../source/index.ts";
import { RecordSource } from "../../source/source.ts";
import { space } from "../space/space.ts";

/** Account or space package restrictions administered in this region. */
export const packagePolicy = table("package_policy", {
    ...recordColumns("package-policy"),
    ...provenanceColumns(),
    /** The account administering the policy, resolved through the global directory. */
    accountId: identifier("account_id", "account").notNull(),
    /** The scope to which every revision applies. */
    scope: text("scope", { enum: ["account", "space"] }).notNull(),
    /** The restricted space; absent for account restrictions. */
    spaceId: identifier("space_id", "space"),
    /** The desired revision generation. */
    generation: integer("generation").notNull().default(1),
    /** The active immutable revision, absent before initial activation. */
    currentRevisionId: identifier("current_revision_id", "package-policy-revision"),
}, (policy) => [
    ...provenanceChecks("package_policy", policy),
    foreignKey({
        columns: [policy.accountId, policy.spaceId],
        foreignColumns: [space.accountId, space.id],
    }).onDelete("restrict"),
    foreignKey({
        columns: [policy.id, policy.currentRevisionId],
        foreignColumns: [packagePolicyRevision.policyId, packagePolicyRevision.id],
    }).onDelete("restrict"),
    uniqueIndex("package_policy_account").on(policy.accountId)
        .where(sql`${policy.scope} = 'account'`),
    uniqueIndex("package_policy_space").on(policy.spaceId)
        .where(sql`${policy.scope} = 'space'`),
    check(
        "package_policy_scope",
        sql`
        (${policy.scope} = 'account' AND ${policy.spaceId} IS NULL) OR
        (${policy.scope} = 'space' AND ${policy.spaceId} IS NOT NULL)
    `,
    ),
    check("package_policy_generation", sql`${policy.generation} > 0`),
]);

/** An immutable package policy definition retained for evaluation and audit. */
export const packagePolicyRevision = table("package_policy_revision", {
    ...recordColumns("package-policy-revision"),
    /** The policy whose scope applies to this definition. */
    policyId: identifier("policy_id", "package-policy").notNull()
        .references((): Column => packagePolicy.id, { onDelete: "restrict" }),
    /** The policy generation that produced this revision. */
    generation: integer("generation").notNull(),
    /** The complete package admission rules. */
    definition: json("definition", PackagePolicyDefinition).notNull(),
    /** The exact source declaration that produced this revision, if source-managed. */
    source: json("source", RecordSource),
    /** SHA-256 of the canonical policy definition. */
    digest: text("digest").notNull(),
}, (revision) => [
    unique("package_policy_revision_policy_id").on(revision.policyId, revision.id),
    unique("package_policy_revision_generation").on(revision.policyId, revision.generation),
    check("package_policy_revision_generation_positive", sql`${revision.generation} > 0`),
    check(
        "package_policy_revision_digest",
        sql`
        length(${revision.digest}) = 64 AND ${revision.digest} NOT GLOB '*[^a-f0-9]*'
    `,
    ),
]);

/** A persisted account or space package policy. */
export type PackagePolicy = Select<typeof packagePolicy>;
/** An immutable policy revision. */
export type PackagePolicyRevision = Select<typeof packagePolicyRevision>;
