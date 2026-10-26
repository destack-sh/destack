import { check, defineTable, identifier, index, integer, text } from "@destack/db";
import { accessRole } from "../role/table.ts";
import { schema } from "@destack/schema";

/** Where an invitation stands: waiting until its expiry, taken, or withdrawn. */
export const INVITATION_STATUSES = ["pending", "accepted", "revoked"] as const;

/** Where an invitation stands. */
export const InvitationStatus = schema.enum(INVITATION_STATUSES);
/** Where an invitation stands. */
export type InvitationStatus = schema.Infer<typeof InvitationStatus>;

/** A pending grant, stored apart from relationships until its addressee or a grantor accepts. */
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
        /** The package declaring the object's type. */
        objectPackageId: identifier("object_package_id", "package").notNull(),
        /** The object's type. */
        objectType: text("object_type").notNull(),
        /** The scope containing the object. */
        objectScope: text("object_scope").notNull(),
        /** The object's identifier. */
        objectId: text("object_id").notNull(),
        /** The offered relation, absent for an offered role binding. */
        relation: text("relation"),
        /** The offered role, absent for an offered relation. */
        roleId: identifier("role_id", "role").references(() => accessRole.id, {
            onDelete: "cascade",
        }),
        /** The package of the principal who invited, absent for the system's invitations. */
        inviterPackageId: identifier("inviter_package_id", "package"),
        /** The type of the principal who invited. */
        inviterType: text("inviter_type"),
        /** The scope of the principal who invited. */
        inviterScope: text("inviter_scope"),
        /** The identifier of the principal who invited. */
        inviterId: text("inviter_id"),
        /** The package of the invited principal, contact or subject set's type. */
        subjectPackageId: identifier("subject_package_id", "package").notNull(),
        /** The type of the invited subject. */
        subjectType: text("subject_type").notNull(),
        /** The scope of the invited subject. */
        subjectScope: text("subject_scope").notNull(),
        /** The identifier of the invited subject. */
        subjectId: text("subject_id").notNull(),
        /** The relation whose members the invited subject set is, absent for one subject. */
        subjectRelation: text("subject_relation"),
        /** The exclusive time the accepted relationship expires, in Unix milliseconds. */
        relationshipExpiresAt: integer("relationship_expires_at"),
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
                invitation.objectPackageId,
                invitation.objectType,
                invitation.objectScope,
                invitation.objectId,
            ),
            index("invitation_subject").on(
                invitation.subjectPackageId,
                invitation.subjectType,
                invitation.subjectScope,
                invitation.subjectId,
            ),
            index("invitation_inviter").on(
                invitation.inviterPackageId,
                invitation.inviterType,
                invitation.inviterScope,
                invitation.inviterId,
            ),
            check("invitation_label", {
                OR: [
                    { relation: { isNull: true }, roleId: { isNotNull: true } },
                    { relation: { isNotNull: true }, roleId: { isNull: true } },
                ],
            }),
            check("invitation_inviter", {
                OR: [
                    {
                        inviterPackageId: { isNull: true },
                        inviterType: { isNull: true },
                        inviterScope: { isNull: true },
                        inviterId: { isNull: true },
                    },
                    {
                        inviterPackageId: { isNotNull: true },
                        inviterType: { isNotNull: true },
                        inviterScope: { isNotNull: true },
                        inviterId: { isNotNull: true },
                    },
                ],
            }),
            check("invitation_status", { status: { in: [...INVITATION_STATUSES] } }),
            check("invitation_expiry", { expiresAt: { gt: { field: "createdAt" } } }),
        ],
    },
);
