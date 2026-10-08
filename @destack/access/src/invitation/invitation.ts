import { type ObjectReference, Subject } from "@destack/sync";
import { and, asc, eq, gt, inArray, type DatabaseConnection, type Select } from "@destack/db";
import { defineSchema, Instant, schema } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import { Contact, type AccessContext, Caller } from "../context/context.ts";
import { Relationship, type RelationshipCondition } from "../relationship/relationship.ts";
import { accessInvitation, InvitationRelationship, InvitationStatus } from "./table.ts";

/** How long an invitation stays acceptable unless it sets its own lapse, seven days in milliseconds. */
export const INVITATION_LIFETIME_MILLISECONDS = 7 * 24 * 60 * 60 * 1000;

/** The most invitations one page lists, a screen of pending invitations at a few hundred bytes each. */
const INVITATION_PAGE_LIMIT = 100;

/** The schema of an invitation: a pending grant of a relationship, which applies once accepted. */
const invitationSchema = defineSchema(
    schema.object({
        /** The stable invitation identifier. */
        id: schema.string().min(1),
        /** The relationship accepting the invitation creates. */
        relationship: InvitationRelationship,
        /** The principal who invited, absent for the system's own invitations. */
        inviter: Subject.nullable(),
        /** Why the inviting principal offers or asks for the relationship. */
        purpose: schema.string().min(1).exactOptional(),
        /** Where the invitation stands. */
        status: InvitationStatus,
        /** The creation time in Unix milliseconds. */
        createdAt: Instant,
        /** The exclusive time the invitation lapses, in Unix milliseconds. */
        expiresAt: Instant,
    }),
);
/** A pending grant: a relationship a grantor offers to a principal or contact, or a principal asks for itself. */
export type Invitation = schema.Infer<typeof invitationSchema>;

/** An invited relationship: offered to a principal or a contact, or asked for by the inviting principal itself. */
export interface InvitationRequest {
    /** The relationship to grant once accepted, its subject the invited principal or contact, or the inviting principal. */
    readonly relationship: {
        /** The related object. */
        readonly object: ObjectReference;
        /** The invited subject: a principal or a contact, or the inviting principal asking for itself. */
        readonly subject: Subject;
        /** Optional expiry in UTC epoch milliseconds. */
        readonly expiresAt?: number;
        /** What a request must satisfy for the relationship to apply. */
        readonly conditions?: RelationshipCondition;
    } & ({ readonly relation: string } | { readonly role: string });
    /** Why the inviting principal offers or asks for the relationship. */
    readonly purpose?: string;
    /** When the invitation lapses in UTC epoch milliseconds, a week from now by default. */
    readonly expiresAt?: number;
}

/** A bounded page of invitations after an identifier. */
export interface InvitationPage {
    /** The identifier the page starts after. */
    readonly after?: string;
    /** The most invitations to read, from 1 to 100. */
    readonly limit: number;
}

/** An invitation: its schema, its rows and its addressees. */
export const Invitation = {
    /** The schema of an invitation. */
    schema: invitationSchema,
    id: invitationId,
    read,
    on,
    pending,
    encode,
    decode,
    asksForItself,
    addresses,
    addressed,
};

/** Select the pending invitations of a page that have not lapsed, after its starting identifier. */
function pending(page: InvitationPage, now: number) {
    // require a bounded page
    if (!Number.isInteger(page.limit) || page.limit < 1 || page.limit > INVITATION_PAGE_LIMIT) {
        throw new AccessError(
            "INVALID_CONTEXT",
            `invitation page limit must be 1 to ${INVITATION_PAGE_LIMIT}`,
        );
    }

    // select pending invitations before their expiry
    return and(
        eq(accessInvitation.status, "pending"),
        gt(accessInvitation.expiresAt, now),
        page.after === undefined ? undefined : gt(accessInvitation.id, invitationId(page.after)),
    );
}

/** Select the invitations on one object. */
function on(object: ObjectReference) {
    return and(
        eq(accessInvitation.objectScope, object.scope),
        eq(accessInvitation.packageId, object.packageId),
        eq(accessInvitation.type, object.type),
        eq(accessInvitation.objectId, object.id),
    );
}

/** Parse an invitation identifier. */
function invitationId(id: string) {
    return schema.identifier("invitation").parse(id);
}

/** Read a pending invitation on an object, refusing one that is no longer pending. */
async function read(
    database: DatabaseConnection,
    object: ObjectReference,
    id: string,
): Promise<Invitation> {
    // read the invitation on the object
    const [row] = await database
        .select()
        .from(accessInvitation)
        .where(and(on(object), eq(accessInvitation.id, invitationId(id))));
    if (!row) {
        throw new AccessError("NOT_FOUND", "invitation not found");
    }

    // refuse a settled invitation
    if (row.status !== "pending") {
        throw new AccessError("CONFLICT", `invitation is ${row.status}`);
    }

    return decode(row);
}

/** Read a page of the pending invitations addressed to some principals or contacts, oldest first. */
async function addressed(
    database: DatabaseConnection,
    addressees: readonly Subject[],
    page: InvitationPage,
    now: number,
): Promise<Invitation[]> {
    const rows = await database
        .select()
        .from(accessInvitation)
        .where(
            and(
                inArray(
                    accessInvitation.addressee,
                    addressees.map((addressee) => Subject.key(addressee)),
                ),
                pending(page, now),
            ),
        )
        .orderBy(asc(accessInvitation.id))
        .limit(page.limit);

    return rows.map(decode);
}

/** Determine whether a principal asked for a relationship with itself, which a grantor accepts. */
function asksForItself(invitation: Invitation): boolean {
    return (
        invitation.inviter !== null &&
        Subject.same(invitation.relationship.subject, invitation.inviter)
    );
}

/** Determine whether an offer names a principal, directly or through a contact it proved. */
function addresses(invitation: Invitation, principal: Subject, context: AccessContext): boolean {
    return [principal, ...Caller.contacts(context).map((contact) => Contact.subject(contact))].some(
        (subject) => Subject.same(subject, invitation.relationship.subject),
    );
}

/** Write an invitation as its row, keyed for listing by inviting principal, addressee and lender. */
function encode(invitation: Invitation, scope: string) {
    const invited = invitation.relationship;
    const lender = invited.conditions?.onBehalfOf;

    return {
        id: invitationId(invitation.id),
        createdAt: invitation.createdAt,
        updatedAt: invitation.createdAt,
        scope,
        objectScope: invited.object.scope,
        packageId: invited.object.packageId,
        type: invited.object.type,
        objectId: invited.object.id,
        ...Relationship.viaColumns(invited),
        relationship: invited,
        inviter: invitation.inviter,
        inviterKey: invitation.inviter === null ? null : Subject.key(invitation.inviter),
        addressee: Subject.key(invited.subject),
        lender: lender === undefined ? null : Subject.key(lender),
        purpose: invitation.purpose ?? null,
        expiresAt: invitation.expiresAt,
        status: invitation.status,
    };
}

/** Read an invitation from its row. */
function decode(row: Select<typeof accessInvitation>): Invitation {
    return {
        id: row.id,
        relationship: row.relationship,
        inviter: row.inviter,
        ...(row.purpose === null ? {} : { purpose: row.purpose }),
        status: InvitationStatus.parse(row.status),
        createdAt: row.createdAt,
        expiresAt: row.expiresAt,
    };
}
