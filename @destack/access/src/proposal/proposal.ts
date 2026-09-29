import { type ObjectReference } from "@destack/sync";
import { and, eq, gt, type DatabaseConnection, type Select } from "@destack/db";
import { defineSchema, identifier, schema } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import { sameSubject, Subject, subjectKey } from "../policy/subject.ts";
import { VerifiedIdentifier, verifiedIdentifiers, type AccessContext } from "../context/context.ts";
import type { RelationshipRequest } from "../relationship/relationship.ts";
import { accessProposal, ProposedRelationship } from "./table.ts";

/** How long a proposal stays acceptable unless it names its own lapse, in milliseconds. */
export const PROPOSAL_LIFETIME_MILLISECONDS = 7 * 24 * 60 * 60 * 1000;

/** The most proposals one page lists, a screen of pending requests at a few hundred bytes each. */
const PROPOSAL_PAGE_LIMIT = 100;

/** The schema of a proposal: a relationship requested or offered, which applies after acceptance. */
const proposalSchema = defineSchema(
    schema.object({
        /** The stable proposal identifier. */
        id: schema.string().min(1),
        /** The relationship accepting the proposal creates. */
        relationship: ProposedRelationship,
        /** The identifier of the owner who may accept in place of a known subject, such as `email:bob@acme.com`. */
        recipient: VerifiedIdentifier.optional(),
        /** The principal proposing the relationship. */
        proposer: Subject,
        /** Why the proposer asks for or offers the relationship. */
        purpose: schema.string().min(1).optional(),
        /** The creation time in Unix milliseconds. */
        createdAt: schema.number().int(),
        /** The exclusive time the proposal lapses, in Unix milliseconds. */
        expiresAt: schema.number().int(),
    }),
);
/** A relationship that its subject requests or a grantor offers, applied after acceptance. */
export type Proposal = schema.Infer<typeof proposalSchema>;

/** A proposed relationship, with exactly one of a subject or a recipient. */
export interface ProposalRequest {
    /** The relationship to propose, with the proposer or the offered principal as subject. */
    readonly relationship: Omit<RelationshipRequest, "subject"> & {
        readonly subject?: Subject;
    };
    /** The identifier of the owner who may accept an offer, such as `email:bob@acme.com`. */
    readonly recipient?: string;
    /** Why the proposer asks for or offers the relationship. */
    readonly purpose?: string;
    /** When the proposal lapses in UTC epoch milliseconds, a week from now by default. */
    readonly expiresAt?: number;
}

/** A bounded page of proposals after an identifier. */
export interface ProposalPage {
    /** The identifier the page starts after. */
    readonly after?: string;
    /** The most proposals to read, from 1 to 100. */
    readonly limit: number;
}

/** A proposal: its schema, its rows and its addressees. */
export const Proposal = {
    /** The schema of a proposal. */
    schema: proposalSchema,
    id: proposalId,
    read,
    on,
    pending,
    encode,
    decode,
    asksForItself,
    addresses,
};

/** Select the proposals of a page that have not lapsed, after its starting identifier. */
function pending(page: ProposalPage, now: number) {
    // require a bounded page
    if (!Number.isInteger(page.limit) || page.limit < 1 || page.limit > PROPOSAL_PAGE_LIMIT) {
        throw new AccessError(
            "INVALID_CONTEXT",
            `proposal page limit must be 1 to ${PROPOSAL_PAGE_LIMIT}`,
        );
    }

    return and(
        gt(accessProposal.expiresAt, now),
        page.after === undefined ? undefined : gt(accessProposal.id, proposalId(page.after)),
    );
}

/** Select the proposals on one object. */
function on(object: ObjectReference) {
    return and(
        eq(accessProposal.objectScope, object.scope),
        eq(accessProposal.packageId, object.packageId),
        eq(accessProposal.type, object.type),
        eq(accessProposal.objectId, object.id),
    );
}

/** Parse a proposal identifier. */
function proposalId(id: string) {
    return identifier("proposal").parse(id);
}

/** Read a pending proposal on an object. */
async function read(
    database: DatabaseConnection,
    object: ObjectReference,
    id: string,
): Promise<Proposal> {
    const [row] = await database
        .select()
        .from(accessProposal)
        .where(and(on(object), eq(accessProposal.id, proposalId(id))));
    if (!row) {
        throw new AccessError("NOT_FOUND", "proposal not found");
    }

    return decode(row);
}

/** Determine whether a principal asked for a relationship with itself. */
function asksForItself(proposal: Proposal): boolean {
    const subject = proposal.relationship.subject;

    return subject !== undefined && sameSubject(subject, proposal.proposer);
}

/** Determine whether an offer addresses a principal directly or through a verified identifier. */
function addresses(proposal: Proposal, principal: Subject, context: AccessContext): boolean {
    const subject = proposal.relationship.subject;

    return subject === undefined
        ? verifiedIdentifiers(context).includes(proposal.recipient!)
        : sameSubject(subject, principal);
}

/** Write a proposal as its row, keyed for listing by proposer, addressee and lender. */
function encode(proposal: Proposal, scope: string) {
    const proposed = proposal.relationship;
    const lender = proposed.conditions?.onBehalfOf;

    return {
        id: proposalId(proposal.id),
        createdAt: proposal.createdAt,
        scope,
        objectScope: proposed.object.scope,
        packageId: proposed.object.packageId,
        type: proposed.object.type,
        objectId: proposed.object.id,
        relation: proposed.relation ?? null,
        roleId: proposed.role === undefined ? null : identifier("role").parse(proposed.role),
        relationship: proposed,
        proposer: proposal.proposer,
        proposerKey: subjectKey(proposal.proposer),
        addressee:
            proposed.subject === undefined ? proposal.recipient! : subjectKey(proposed.subject),
        lender: lender === undefined ? null : subjectKey(lender),
        purpose: proposal.purpose ?? null,
        expiresAt: proposal.expiresAt,
    };
}

/** Read a proposal from its row, deriving the recipient of an offer to whoever proves it. */
function decode(row: Select<typeof accessProposal>): Proposal {
    return {
        id: row.id,
        relationship: row.relationship,
        ...(row.relationship.subject === undefined ? { recipient: row.addressee } : {}),
        proposer: row.proposer,
        ...(row.purpose === null ? {} : { purpose: row.purpose }),
        createdAt: row.createdAt,
        expiresAt: row.expiresAt,
    };
}
