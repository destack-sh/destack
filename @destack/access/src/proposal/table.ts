import { check, defineTable, identifier, index, integer, json, sql, text } from "@destack/db";
import { AccessName, Subject } from "@destack/sync";
import { accessRole } from "../role/table.ts";
import { defineSchema, schema } from "@destack/schema";
import { relationshipBase } from "../relationship/relationship.ts";

/** A relationship as proposed, before anyone accepts it: through a declared relation, or binding a role. */
export const ProposedRelationship = defineSchema(
    schema.union([
        relationshipBase.omit({ id: true, createdAt: true }).extend({
            /** The declared relation. */
            relation: AccessName,
        }),
        relationshipBase.omit({ id: true, createdAt: true }).extend({
            /** The bound role. */
            role: schema.string().min(1),
        }),
    ]),
);
/** A relationship as proposed, before anyone accepts it. */
export type ProposedRelationship = schema.Infer<typeof ProposedRelationship>;

/** A relationship awaiting acceptance, kept apart from the relationships that apply. */
export const accessProposal = defineTable(
    "proposal",
    {
        /** The proposal identifier. */
        id: identifier("id", "proposal").primaryKey(),
        /** Creation time in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** The scope the proposal lives in and routes changes to. */
        scope: text("scope").notNull(),
        /** The scope containing the object. */
        objectScope: text("object_scope").notNull(),
        /** The package declaring the object's type. */
        packageId: identifier("package_id", "package").notNull(),
        /** The object's type. */
        type: text("type").notNull(),
        /** The object's identifier. */
        objectId: text("object_id").notNull(),
        /** The proposed relation, absent for a proposed role binding. */
        relation: text("relation"),
        /** The proposed role, absent for a proposed relation. */
        roleId: identifier("role_id", "role").references(() => accessRole.id, {
            onDelete: "cascade",
        }),
        /** The relationship accepting the proposal creates. */
        relationship: json("relationship", ProposedRelationship).notNull(),
        /** The principal proposing the relationship. */
        proposer: json("proposer", Subject).notNull(),
        /** The proposer's key, listing what a principal proposed. */
        proposerKey: text("proposer_key").notNull(),
        /** The proposed subject's key or the recipient identifier, listing what addresses a principal. */
        addressee: text("addressee").notNull(),
        /** The key of the principal a proposed delegation lends from, listing what awaits its consent. */
        lender: text("lender"),
        /** Why the proposer asks for or offers the relationship. */
        purpose: text("purpose"),
        /** The exclusive time the proposal lapses, in Unix milliseconds. */
        expiresAt: integer("expires_at").notNull(),
    },
    {
        log: { retention: "history" },
        constraints: (proposal) => [
            index("proposal_object").on(
                proposal.objectScope,
                proposal.packageId,
                proposal.type,
                proposal.objectId,
            ),
            index("proposal_addressee").on(proposal.addressee),
            index("proposal_proposer").on(proposal.proposerKey),
            index("proposal_lender").on(proposal.lender),
            check(
                "proposal_label",
                sql`(${proposal.relation} IS NULL) <> (${proposal.roleId} IS NULL)`,
            ),
            check("proposal_expiry", sql`${proposal.expiresAt} > ${proposal.createdAt}`),
        ],
    },
);
