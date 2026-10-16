import { v7 } from "uuid";
import { Table } from "@destack/db";
import * as policies from "../declare/principal.ts";
import { Authorization } from "../authorizer/authorization.ts";
import { AccessName, ObjectReference, Subject } from "@destack/sync";
import {
    and,
    Condition,
    eq,
    type Transaction,
    type Select,
    type Snapshot,
    type SQL,
} from "@destack/db";
import { defineSchema, type Identifier, Instant, schema } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import { Contact, AccessContext, Caller } from "../context/context.ts";
import {
    Relationship,
    relationshipBase,
    type RelationshipCondition,
} from "../relationship/relationship.ts";
import { accessInvitation, InvitationStatus } from "./table.ts";

/** A relationship an invitation offers or asks for: by a declared relation, or a role binding. */
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

/** How long an invitation stays acceptable without its own expiry: seven days, in milliseconds. */
export const INVITATION_LIFETIME_MILLISECONDS = 7 * 24 * 60 * 60 * 1000;

/** The most invitations per page: a screen of pending invitations of a few hundred bytes. */
export const INVITATION_PAGE = 100;

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
/** A pending grant: a relationship offered to a principal or contact, or asked for by one. */
export type Invitation = schema.Infer<typeof invitationSchema>;

/** An invited relationship: offered to a principal or contact, or asked for by the inviter. */
export interface InvitationRequest {
    /** The relationship to grant on acceptance, for the invited subject or the inviter. */
    readonly relationship: {
        /** The related object. */
        readonly object: ObjectReference;
        /** The invited subject: a principal or contact, or the inviter asking for itself. */
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

    /** Match a page's unexpired pending invitations, after its starting identifier. */
    pending(page: InvitationPage, now: number): Condition<Select<typeof accessInvitation>> {
        // require a bounded page
        if (!Number.isInteger(page.limit) || page.limit < 1 || page.limit > INVITATION_PAGE) {
            throw new AccessError(
                "INVALID_CONTEXT",
                `invitation page limit must be 1 to ${INVITATION_PAGE}`,
            );
        }

        // match pending invitations before their expiry
        return {
            status: "pending",
            expiresAt: { gt: now },
            ...(page.after === undefined ? {} : { id: { gt: Invitation.id(page.after) } }),
        };
    },

    /** Select the invitations on one object. */
    on(object: ObjectReference): SQL | undefined {
        return and(
            eq(accessInvitation.objectScope, object.scope),
            eq(accessInvitation.objectPackageId, object.packageId),
            eq(accessInvitation.objectType, object.type),
            eq(accessInvitation.objectId, object.id),
        );
    },

    /** Parse an invitation identifier. */
    id(id: string): Identifier<"invitation"> {
        return schema.identifier("invitation").parse(id);
    },

    /** Read a pending invitation on an object in a transaction, refusing one no longer pending. */
    async read(transaction: Transaction, object: ObjectReference, id: string): Promise<Invitation> {
        // read the invitation on the object
        const row = await transaction.read(accessInvitation).findFirst({
            where: {
                ...Relationship.objectColumns(object),
                id: Invitation.id(id),
            },
        });
        if (!row) {
            throw new AccessError("NOT_FOUND", "invitation not found");
        }

        // refuse a settled invitation
        if (row.status !== "pending") {
            throw new AccessError("CONFLICT", `invitation is ${row.status}`);
        }

        return Invitation.decode(row);
    },

    /** Read the invitations on an object to one principal or contact, in any status. */
    async readOn(
        snapshot: Snapshot,
        object: ObjectReference,
        addressee: Subject,
    ): Promise<Invitation[]> {
        const rows = await snapshot.read(accessInvitation).findMany({
            where: {
                ...Relationship.objectColumns(object),
                ...Relationship.subjectCondition(addressee),
            },
        });

        return rows.map((row) => Invitation.decode(row));
    },

    /** Read a page of the pending invitations to some principals or contacts, oldest first. */
    async addressed(
        reader: Pick<Snapshot, "read">,
        addressees: readonly Subject[],
        page: InvitationPage,
        now: number,
    ): Promise<Invitation[]> {
        // read the addressees' pending invitations in identifier order, one page
        const rows = await reader.read(accessInvitation).findMany({
            where: {
                AND: [
                    {
                        OR: addressees.map((addressee) => Relationship.subjectCondition(addressee)),
                    },
                    Invitation.pending(page, now),
                ],
            },
            orderBy: { id: "asc" },
            limit: page.limit,
        });

        return rows.map((row) => Invitation.decode(row));
    },

    /** Report whether a principal asked for a relationship with itself, for a grantor to accept. */
    asksForItself(invitation: Invitation): boolean {
        return (
            invitation.inviter !== null &&
            Subject.same(invitation.relationship.subject, invitation.inviter)
        );
    },

    /** Report whether an offer is addressed to a principal, directly or by a proved contact. */
    addresses(invitation: Invitation, principal: Subject, context: AccessContext): boolean {
        return [
            principal,
            ...Caller.contacts(context).map((contact) => Contact.subject(contact)),
        ].some((subject) => Subject.same(subject, invitation.relationship.subject));
    },

    /** Write an invitation as its row, keyed for listing by inviter and addressee. */
    encode(invitation: Invitation, scope: string): Select<typeof accessInvitation> {
        const invited = invitation.relationship;

        return {
            id: Invitation.id(invitation.id),
            createdAt: invitation.createdAt,
            updatedAt: invitation.createdAt,
            scope,
            ...Relationship.objectColumns(invited.object),
            ...Relationship.viaColumns(invited),
            ...Relationship.subjectColumns(invited.subject),
            relationshipExpiresAt: invited.expiresAt,
            ...Relationship.conditionColumns(invited.conditions),
            ...ObjectReference.columns("inviter", invitation.inviter ?? undefined),
            purpose: invitation.purpose ?? null,
            expiresAt: invitation.expiresAt,
            status: invitation.status,
        };
    },

    /** Read an invitation from its row. */
    decode(row: Select<typeof accessInvitation>): Invitation {
        return {
            id: row.id,
            relationship: {
                object: ObjectReference.at("object", row),
                ...Relationship.viaRelation(row),
                subject: Relationship.subjectOf(row),
                expiresAt: row.relationshipExpiresAt,
                ...Relationship.conditionsOf(row),
            },
            inviter: row.inviterId === null ? null : ObjectReference.at("inviter", row),
            ...(row.purpose === null ? {} : { purpose: row.purpose }),
            status: InvitationStatus.parse(row.status),
            createdAt: row.createdAt,
            expiresAt: row.expiresAt,
        };
    },

    /** Invite a relationship that applies once accepted, asked by its subject or offered. */
    async create(request: InvitationRequest, authorization: Authorization): Promise<Invitation> {
        return authorization.transaction.savepoint(async () => {
            // require a local object and an inviting principal or the system
            const invited = request.relationship;
            await authorization.authorizer.requireLocal(authorization.transaction, invited.object);
            const context = authorization.context();
            const inviter = Caller.principal(context) ?? null;

            // validate the relationship, checking an invited contact's principal once it accepts
            const { subject, ...validated } = invited;
            authorization.authorizer.validate(
                policies.principal.contact.is(subject) ? validated : invited,
                context.now,
            );
            const expiresAt = request.expiresAt ?? context.now + INVITATION_LIFETIME_MILLISECONDS;
            if (!Number.isFinite(expiresAt) || expiresAt <= context.now) {
                throw new AccessError("FORBIDDEN", "invitation must expire in the future");
            }

            // let a principal ask for itself, and require grant authority to offer to anyone else
            if (inviter === null || !Subject.same(invited.subject, inviter)) {
                if (
                    invited.subject.relation !== undefined ||
                    invited.subject.id === "*" ||
                    invited.subject.scope === "*"
                ) {
                    throw new AccessError("FORBIDDEN", "invitation subject must be one principal");
                }
                await authorization.authorizeGrant(invited);
            }

            // store the invitation apart from the relationships that apply
            const invitation: Invitation = {
                id: schema.identifier("invitation").parse(`invitation-${v7()}`),
                relationship: {
                    object: invited.object,
                    ...Relationship.via(invited),
                    subject: invited.subject,
                    expiresAt: invited.expiresAt ?? null,
                    ...(invited.conditions === undefined ? {} : { conditions: invited.conditions }),
                },
                inviter,
                ...(request.purpose === undefined ? {} : { purpose: request.purpose }),
                status: "pending",
                createdAt: context.now,
                expiresAt,
            };
            const scope = authorization.authorizer.governingScope(invited.object);
            const row = Table.complete(accessInvitation, Invitation.encode(invitation, scope));
            await authorization.transaction.write({
                table: accessInvitation,
                before: null,
                after: row,
            });

            return invitation;
        });
    },

    /** Accept an invitation on an object so its relationship applies. */
    async accept(
        object: ObjectReference,
        id: string,
        authorization: Authorization,
    ): Promise<Relationship> {
        return authorization.transaction.savepoint(async () => {
            // read the pending invitation
            const context = authorization.context();
            const invitation = await Invitation.read(authorization.transaction, object, id);
            if (invitation.expiresAt <= context.now) {
                throw new AccessError("FORBIDDEN", "invitation has lapsed");
            }

            // require a grantor for a request
            const invited = invitation.relationship;
            let subject: Subject;
            if (Invitation.asksForItself(invitation)) {
                await authorization.authorizeGrant(invited);
                subject = invitation.relationship.subject;
            }
            // require the addressed principal for an offer, and an inviter that still has the grant
            else {
                const accepting = Caller.principal(context);
                if (Caller.isAnonymous(context)) {
                    throw new AccessError("FORBIDDEN", "accepting an offer needs an identity");
                } else if (!Invitation.addresses(invitation, accepting, context)) {
                    throw new AccessError("FORBIDDEN", "invitation is addressed to someone else");
                }
                subject = accepting;
                await requireInviter(invitation, context.now, authorization);
            }

            // relate the accepting subject and settle the invitation
            const { expiresAt, ...granted } = invited;
            const relationship = await Relationship.create(
                {
                    ...granted,
                    subject,
                    ...(expiresAt === null ? {} : { expiresAt }),
                },
                authorization,
            );
            await settle(id, "accepted", context.now, authorization);
            authorization.renew();

            return relationship;
        });
    },

    /** Withdraw an invitation on an object as its inviter, addressee, or a revoking grantor. */
    async withdraw(
        object: ObjectReference,
        id: string,
        authorization: Authorization,
    ): Promise<Invitation> {
        return authorization.transaction.savepoint(async () => {
            // let the inviting principal and the addressee withdraw without grant authority
            const context = authorization.context();
            const invitation = await Invitation.read(authorization.transaction, object, id);
            const acting = Caller.principal(context);
            const isInviter =
                acting !== undefined &&
                invitation.inviter !== null &&
                Subject.same(acting, invitation.inviter);
            if (
                acting === undefined ||
                (!isInviter && !Invitation.addresses(invitation, acting, context))
            ) {
                await authorization.authorizeRevoke(invitation.relationship);
            }

            // mark the invitation revoked
            await settle(id, "revoked", context.now, authorization);

            return { ...invitation, status: "revoked" };
        });
    },

    /** List a page of an object's pending invitations by identifier, as its grantors may. */
    async list(
        request: {
            readonly object: ObjectReference;
            readonly relation?: string;
            readonly role?: string;
        },
        page: InvitationPage,
        authorization: Authorization,
    ): Promise<Invitation[]> {
        // require the grant permission
        const context = authorization.context();
        await authorization.requireGrant(request.object, request.relation);

        // read the object's invitations after the cursor
        const role =
            request.role === undefined ? undefined : schema.identifier("role").parse(request.role);
        const rows = await authorization.transaction.read(accessInvitation).findMany({
            where: {
                ...Relationship.objectColumns(request.object),
                ...(request.relation === undefined ? {} : { relation: request.relation }),
                ...(role === undefined ? {} : { roleId: role }),
                ...Invitation.pending(page, context.now),
            },
            orderBy: { id: "asc" },
            limit: page.limit + 1,
        });

        return rows.map((row) => Invitation.decode(row));
    },
};

/** Require an offer's inviter to still have the grant, except for system offers. */
async function requireInviter(
    invitation: Invitation,
    now: number,
    authorization: Authorization,
): Promise<void> {
    // accept system offers as made
    if (invitation.inviter === null) {
        return;
    }

    // decide the grant as the inviting principal, at its authentication when it sent the offer
    const inviter = invitation.inviter;
    const inviting = new Authorization(authorization.authorizer, authorization.transaction, () =>
        AccessContext.of(Caller.of(inviter), now),
    );
    await inviting.authorizeGrant(invitation.relationship);
}

/** Settle a pending invitation as accepted or revoked, refusing one settled first. */
async function settle(
    id: string,
    status: "accepted" | "revoked",
    now: number,
    authorization: Authorization,
): Promise<void> {
    // refuse an invitation no longer pending
    const row = await authorization.transaction
        .read(accessInvitation)
        .findFirst({ where: { id: Invitation.id(id) } });
    if (row === undefined || row.status !== "pending") {
        throw new AccessError("CONFLICT", "invitation is no longer pending");
    }

    // mark it settled
    await authorization.transaction.write({
        table: accessInvitation,
        before: row,
        after: { ...row, status, updatedAt: now },
    });
}
