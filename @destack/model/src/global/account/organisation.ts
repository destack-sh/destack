import {
    check,
    identifier,
    index,
    integer,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";
import { user } from "./user.ts";

/** An organisation administering one or more accounts. */
export const organisation = table("organisation", {
    ...recordColumns("organisation"),
    /** The displayed organisation name. */
    name: text("name").notNull(),
    /** The organisation image URL. */
    image: text("image"),
    /** The time the organisation requested deletion. */
    deletionRequestedAt: integer("deletion_requested_at"),
});

/** A user's membership and administrative authority in an organisation. */
export const organisationMembership = table(
    "organisation_membership",
    {
        ...recordColumns("organisation-membership"),
        /** The organisation admitting the user. */
        organisationId: identifier("organisation_id", "organisation")
            .notNull()
            .references(() => organisation.id, { onDelete: "cascade" }),
        /** The admitted user. */
        userId: identifier("user_id", "user")
            .notNull()
            .references(() => user.id, { onDelete: "restrict" }),
        /** The organisation-level administrative role. */
        role: text("role", { enum: ["owner", "admin", "member"] }).notNull(),
    },
    (membership) => [
        unique("organisation_membership_user").on(membership.organisationId, membership.userId),
        index("organisation_membership_user_id").on(membership.userId),
        check(
            "organisation_membership_role",
            sql`${membership.role} IN ('owner', 'admin', 'member')`,
        ),
    ],
);

/** A single-use invitation to join an organisation. */
export const organisationInvitation = table(
    "organisation_invitation",
    {
        ...recordColumns("organisation-invitation"),
        /** The organisation accepting a new member. */
        organisationId: identifier("organisation_id", "organisation")
            .notNull()
            .references(() => organisation.id, { onDelete: "cascade" }),
        /** The user issuing the invitation. */
        invitedBy: identifier("invited_by", "user")
            .notNull()
            .references(() => user.id),
        /** The email address the accepting user must verify. */
        email: text("email").notNull(),
        /** The administrative role granted on acceptance. */
        role: text("role", { enum: ["owner", "admin", "member"] }).notNull(),
        /** The hash of the invitation credential. */
        tokenHash: text("token_hash").notNull().unique(),
        /** The acceptance deadline in epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
        /** The user accepting the invitation. */
        acceptedBy: identifier("accepted_by", "user").references(() => user.id),
        /** The acceptance time in epoch milliseconds. */
        acceptedAt: integer("accepted_at"),
        /** The withdrawal time in epoch milliseconds. */
        revokedAt: integer("revoked_at"),
    },
    (invitation) => [
        index("organisation_invitation_organisation").on(invitation.organisationId),
        check(
            "organisation_invitation_role",
            sql`${invitation.role} IN ('owner', 'admin', 'member')`,
        ),
        check(
            "organisation_invitation_expiry",
            sql`${invitation.expiresAt} > ${invitation.createdAt}`,
        ),
        check(
            "organisation_invitation_acceptance",
            sql`(${invitation.acceptedBy} IS NULL) = (${invitation.acceptedAt} IS NULL) AND (${invitation.acceptedAt} IS NULL OR (${invitation.revokedAt} IS NULL AND ${invitation.acceptedAt} >= ${invitation.createdAt} AND ${invitation.acceptedAt} < ${invitation.expiresAt}))`,
        ),
    ],
);

/** A persisted organisation. */
export type Organisation = Select<typeof organisation>;
/** A persisted organisation membership. */
export type OrganisationMembership = Select<typeof organisationMembership>;
/** A persisted organisation invitation. */
export type OrganisationInvitation = Select<typeof organisationInvitation>;
