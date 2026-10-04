import { check, defineTable, identifier, index, integer, json, sql, text } from "@destack/db";
import { AccessName, Subject } from "@destack/sync";
import { accessRole } from "../role/table.ts";
import { defineSchema, schema } from "@destack/schema";
import { relationshipBase } from "../relationship/relationship.ts";

/** Where an invitation stands: waiting, taken, withdrawn or lapsed. */
export const INVITATION_STATUSES = ["pending", "accepted", "revoked", "expired"] as const;

/** The statuses as SQL string literals, which the status check lists. */
const STATUS_LITERALS = INVITATION_STATUSES.map((status) => `'${status}'`).join(", ");

/** Where an invitation stands. */
export const InvitationStatus = defineSchema(schema.enum(INVITATION_STATUSES));
/** Where an invitation stands. */
export type InvitationStatus = schema.Infer<typeof InvitationStatus>;

/** A relationship an invitation offers or asks for: through a declared relation, or binding a role. */
export const InvitationRelationship = defineSchema(
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
/** A relationship an invitation offers or asks for. */
export type InvitationRelationship = schema.Infer<typeof InvitationRelationship>;

/** A pending grant, kept apart from the relationships that apply until its addressee or a grantor accepts it. */
export const accessInvitation = defineTable(
    "invitation",
    {
        /** The invitation identifier. */
        id: identifier("id", "invitation").primaryKey(),
        /** Creation time in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** The time the status last changed, in UTC epoch milliseconds. */
        updatedAt: integer("updated_at").notNull(),
        /** The scope the invitation lives in and routes changes to. */
        scope: text("scope").notNull(),
        /** The scope containing the object. */
        objectScope: text("object_scope").notNull(),
        /** The package declaring the object's type. */
        packageId: identifier("package_id", "package").notNull(),
        /** The object's type. */
        type: text("type").notNull(),
        /** The object's identifier. */
        objectId: text("object_id").notNull(),
        /** The offered relation, absent for an offered role binding. */
        relation: text("relation"),
        /** The offered role, absent for an offered relation. */
        roleId: identifier("role_id", "role").references(() => accessRole.id, {
            onDelete: "cascade",
        }),
        /** The relationship accepting the invitation creates. */
        relationship: json("relationship", InvitationRelationship).notNull(),
        /** The principal who invited, absent for the system's own invitations. */
        inviter: json("inviter", Subject),
        /** The inviting principal's key, listing what a principal offered or asked for. */
        inviterKey: text("inviter_key"),
        /** The invited subject's key, a principal or a contact, listing what addresses a principal. */
        addressee: text("addressee").notNull(),
        /** The key of the principal an invited delegation lends from, listing what awaits its consent. */
        lender: text("lender"),
        /** Why the inviting principal offers or asks for the relationship. */
        purpose: text("purpose"),
        /** The exclusive time the invitation lapses, in Unix milliseconds. */
        expiresAt: integer("expires_at").notNull(),
        /** Where the invitation stands. */
        status: text("status").validate(InvitationStatus).notNull().default("pending"),
    },
    {
        log: { retention: "history" },
        constraints: (invitation) => [
            index("invitation_object").on(
                invitation.objectScope,
                invitation.packageId,
                invitation.type,
                invitation.objectId,
            ),
            index("invitation_addressee").on(invitation.addressee),
            index("invitation_inviter").on(invitation.inviterKey),
            index("invitation_lender").on(invitation.lender),
            check(
                "invitation_label",
                sql`(${invitation.relation} IS NULL) <> (${invitation.roleId} IS NULL)`,
            ),
            check(
                "invitation_inviter",
                sql`(${invitation.inviter} IS NULL) = (${invitation.inviterKey} IS NULL)`,
            ),
            check("invitation_status", sql`${invitation.status} IN (${sql.raw(STATUS_LITERALS)})`),
            check("invitation_expiry", sql`${invitation.expiresAt} > ${invitation.createdAt}`),
        ],
    },
);
