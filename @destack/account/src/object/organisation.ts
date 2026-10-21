import type { Select } from "@destack/db";
import {
    Invitation,
    none,
    permission,
    principal,
    relation,
    Relationship,
    union,
} from "@destack/access";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { StackSource } from "./stack-source.ts";
import { AccountHandle } from "../address/handle.ts";
import { residency, ResidencyCode } from "./residency.ts";
import { sudo } from "../authentication/sudo.ts";
import { DisplayName } from "./display-name.ts";

/** The longest invitation purpose, in characters: a short paragraph. */
const PURPOSE_LENGTH = 1000;

/** The roles a member has in its organisation, from the most to the least access. */
export const MEMBER_ROLES = ["owner", "admin", "billing", "member", "guest"] as const;

/** The role a member joins with when none is given. */
export const DEFAULT_MEMBER_ROLE = "member";

/** A role a member has in its organisation. */
export const MemberRole = schema.enum(MEMBER_ROLES);
/** A role a member has in its organisation. */
export type MemberRole = schema.Infer<typeof MemberRole>;

/** The relation to the users of active members in a role below owner. */
const SEATS = { subjects: [principal.user], grantedBy: "manage" };

/** The input selecting one invitation to the organisation. */
const InvitationInput = schema.object({
    /** The invitation's identifier. */
    invitationId: schema.identifier("invitation"),
});

/** An organisation that owns accounts and has members in its roles. */
export const organisation = defineObject(
    {
        name: "organisation",
        plural: "organisations",
        scope: "universe",
        isScope: true,
        fields: {
            /** The display name. */
            name: field.string(DisplayName),
            /** The logo image URL. */
            image: field.string(schema.url()).optional(),
            /** What the organisation is: a business, a family, a community or a school. */
            kind: field.enum(["business", "family", "community", "education"]).default("business"),
            /** The account the charges of the organisation's accounts consolidate to. */
            payer: field.string(schema.identifier("account")).guardWrite("bill"),
        },
        recoverable: { within: { days: 30 }, by: "delete" },
        relations: {
            // relate each active member's user by its role, and the machines publishing as the platform
            owner: { ...SEATS, grantedBy: "own" },
            admin: {
                subjects: [principal.user, residency.members("machine")],
                grantedBy: "manage",
            },
            billing: SEATS,
            member: SEATS,
            guest: SEATS,
            // relate the servers of each residency of its accounts
            resident: { subjects: [residency.members("server")], grantedBy: "place" },
        },
        permissions: {
            read: union(
                relation("owner"),
                relation("admin"),
                relation("billing"),
                relation("member"),
                relation("guest"),
            ),
            view: union(relation("owner"), relation("admin"), relation("member")),
            manage: union(relation("owner"), relation("admin")),
            bill: union(relation("owner"), relation("billing")),
            own: relation("owner"),
            list: union(permission("read"), relation("resident")),
            replicate: relation("resident"),
            update: permission("manage"),
            delete: permission("own"),
            place: none(),
        },
        reserved: ["own", "replicate", "place"],
        elevated: { delete: sudo, own: sudo },
    },
    (method) => ({
        list: method.list("list"),
        create: method.create("update", {
            fields: ["name", "image", "kind"],
            input: schema.object({
                /** The handle the organisation's account claims. */
                handle: AccountHandle,
                /** The jurisdiction the organisation's account stores data in. */
                residencyId: ResidencyCode,
                /** The stack the home of the organisation's account carries, none when absent. */
                stack: StackSource.exactOptional(),
            }),
            isPredicted: false,
        }),
        update: method.update("update", { fields: ["name", "image", "kind", "payer"] }),

        // change invitations under the invitation's own access
        invite: method.mutation({
            permission: "manage",
            target: "id",
            input: schema.object({
                /** The role of the accepted seat. */
                relation: MemberRole,
                /** The invited user, or the contact of the invited email. */
                subject: Subject,
                /** Why the organisation invites. */
                purpose: schema.string().min(1).max(PURPOSE_LENGTH).exactOptional(),
            }),
            output: Invitation.schema,
        }),
        accept: method.mutation({
            permission: "read",
            target: "id",
            input: InvitationInput,
            output: Relationship.schema,
        }),
    }),
);
/** A persisted organisation. */
export type Organisation = Select<typeof organisation.table>;
