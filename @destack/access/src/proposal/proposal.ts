import { type ObjectReference, Subject } from "@destack/sync";
import { and, eq, gt, type DatabaseConnection, type Select } from "@destack/db";
import { defineSchema, Instant, schema } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import { Contact, AccessContext, Caller } from "../context/context.ts";
import { Relationship, type RelationshipCondition } from "../relationship/relationship.ts";
import { accessProposal, ProposedRelationship } from "./table.ts";

/** How long a proposal stays acceptable unless it sets its own lapse, in milliseconds. */
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
        /** The principal proposing the relationship. */
        proposer: Subject,
        /** Why the proposer asks for or offers the relationship. */
        purpose: schema.string().min(1).exactOptional(),
        /** The creation time in Unix milliseconds. */
        createdAt: Instant,
        /** The exclusive time the proposal lapses, in Unix milliseconds. */
        expiresAt: Instant,
    }),
);
/** A relationship that its subject requests or a grantor offers, applied after acceptance. */
export type Proposal = schema.Infer<typeof proposalSchema>;

/** A proposed relationship, its subject the proposer asking for itself, or the principal or contact offered it. */
export interface ProposalRequest {
    /** The relationship to propose, with the proposer or the offered principal as subject. */
    readonly relationship: {
        /** The related object. */
        readonly object: ObjectReference;
        /** The proposed subject: the proposer, or the principal or contact offered the relationship. */
        readonly subject: Subject;
        /** Optional expiry in UTC epoch milliseconds. */
        readonly expiresAt?: number;
        /** What a request must satisfy for the relationship to apply. */
        readonly conditions?: RelationshipCondition;
    } & ({ readonly relation: string } | { readonly role: string });
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
    return schema.identifier("proposal").parse(id);
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

/** Determine whether a principal asked for a relationship with itself, which a grantor accepts. */
function asksForItself(proposal: Proposal): boolean {
    return Subject.same(proposal.relationship.subject, proposal.proposer);
}

/** Determine whether an offer names a principal, directly or through a contact it proved. */
function addresses(proposal: Proposal, principal: Subject, context: AccessContext): boolean {
    return [principal, ...Caller.contacts(context).map((contact) => Contact.subject(contact))].some(
        (subject) => Subject.same(subject, proposal.relationship.subject),
    );
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
        ...Relationship.viaColumns(proposed),
        relationship: proposed,
        proposer: proposal.proposer,
        proposerKey: Subject.key(proposal.proposer),
        addressee: Subject.key(proposed.subject),
        lender: lender === undefined ? null : Subject.key(lender),
        purpose: proposal.purpose ?? null,
        expiresAt: proposal.expiresAt,
    };
}

/** Read a proposal from its row. */
function decode(row: Select<typeof accessProposal>): Proposal {
    return {
        id: row.id,
        relationship: row.relationship,
        proposer: row.proposer,
        ...(row.purpose === null ? {} : { purpose: row.purpose }),
        createdAt: row.createdAt,
        expiresAt: row.expiresAt,
    };
}
