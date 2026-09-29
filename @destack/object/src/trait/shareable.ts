import {
    AccessName,
    Proposal,
    Relationship,
    RelationshipCondition,
    Subject,
    VerifiedIdentifier,
    Explanation,
} from "@destack/access";
import { and, eq, type Column, type Table } from "@destack/db";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Page, page } from "@destack/service/page";
import type { Call } from "../method/call.ts";
import { Step } from "../method/step.ts";
import type * as sync from "@destack/sync";
import { defineMethod, type Method, type MethodKind } from "../method/method.ts";
import {
    Empty,
    PageShape,
    type MethodProcedure,
    type ObjectSchema,
    type Procedure,
    type ReplayShape,
    type Route,
    type TargetShape,
} from "../method/procedure.ts";
import type { ObjectType } from "../object/object.ts";
import type { Trait } from "./trait.ts";

/** The fields relating a subject to an object through exactly one relation or role. */
const GrantShape = {
    /** The declared relation to grant. */
    relation: AccessName.optional(),
    /** The role to bind. */
    role: schema.string().min(1).optional(),
    /** The subject, subject set or wildcard. */
    subject: Subject,
    /** Optional expiry in UTC epoch milliseconds. */
    expiresAt: schema.number().int().optional(),
    /** What a request must satisfy for the relationship to apply. */
    conditions: RelationshipCondition.optional(),
};

/** A relationship an object's sharing methods grant. */
const GrantInput = schema.object(GrantShape);

/** The fields proposing a relationship to exactly one subject or recipient. */
const ProposeShape = {
    /** The relationship to propose. */
    relationship: schema.object({ ...GrantShape, subject: Subject.optional() }),
    /** The identifier whose owner may accept an offer, such as `email:bob@acme.com`. */
    recipient: VerifiedIdentifier.optional(),
    /** Why the caller asks for or offers the relationship. */
    purpose: schema.string().min(1).max(1000).optional(),
    /** When the proposal lapses in UTC epoch milliseconds, a week from now by default. */
    expiresAt: schema.number().int().optional(),
};

/** A relationship an object's sharing methods propose. */
const ProposeInput = schema.object(ProposeShape);

/** The field selecting one relationship. */
const RelationshipShape = {
    /** The relationship's identifier. */
    relationshipId: schema.string().min(1),
};

/** The fields selecting what an explanation covers. */
const ExplainShape = {
    /** The permission to explain, reading by default. */
    permission: AccessName.optional(),
    /** The subject whose access to explain, the caller by default. */
    subject: Subject.optional(),
};

/** The field selecting one proposal. */
const ProposalShape = {
    /** The proposal's identifier. */
    proposalId: schema.string().min(1),
};

/** The methods sharing derives from an object's grant permission. */
export type ShareableMethodMap<Grant> = [Grant] extends [string]
    ? {
          readonly relationships: Method<"relationships", Grant, never, never, false>;
          readonly grant: Method<"grant", null, never, never, true>;
          readonly revoke: Method<"revoke", null, never, never, true>;
          readonly proposals: Method<"proposals", Grant, never, never, false>;
          readonly propose: Method<"propose", null, never, never, true>;
          readonly accept: Method<"accept", null, never, never, true>;
          readonly decline: Method<"decline", null, never, never, true>;
          readonly explain: Method<"explain", null, never, never, false>;
      }
    : {};

/** Objects callers share through relationships and proposals. */
export const shareable: Trait<string> = {
    key: "shareable",
    isDurable: true,
    options: (definition) => definition.shareable?.by,
    columns: () => ({}),
    constraints: () => [],
    methods: (grant) => ({
        relationships: sharingMethod(
            "relationships",
            grant,
            { method: "GET", path: "/{id}/relationships" },
            (shapes) => shapes.target.extend(PageShape),
            page(Relationship.schema),
            async (call) => {
                // page the object's relationships by identifier
                const listing = sharingPage(call, "relationships");
                const relationships = await call.authorization!.authorizer.relationships(
                    call.authorization!.snapshot,
                    call.reference(),
                    {
                        ...(listing.after === undefined ? {} : { after: listing.after }),
                        limit: listing.limit + 1,
                    },
                );

                return listing.result(relationships, (relationship) => relationship.id);
            },
        ),
        grant: sharingMethod(
            "grant",
            null,
            { method: "POST", path: "/{id}/relationships" },
            (shapes) => shapes.target.extend({ ...shapes.replay, ...GrantShape }),
            Relationship.schema,
            async (call) => {
                // grant only on an object outside the trash
                await requirePresent(call, "live");

                return call.authorization!.grant({
                    ...GrantInput.parse(call.input),
                    object: call.reference(),
                });
            },
            (step) => {
                // revoke the granted relationship
                const granted = (step.result as { readonly id?: unknown } | undefined)?.id;
                const call =
                    granted === undefined
                        ? undefined
                        : Step.call(step, "revoke", {
                              ...Step.target(step, step.input.id),
                              relationshipId: granted,
                          });

                return call === undefined ? undefined : [call];
            },
        ),
        revoke: sharingMethod(
            "revoke",
            null,
            { method: "DELETE", path: "/{id}/relationships/{relationshipId}" },
            (shapes) => shapes.target.extend({ ...shapes.replay, ...RelationshipShape }),
            Empty,
            async (call) => {
                // revoke on any object that exists, in the trash too
                await requirePresent(call, "held");
                await call.authorization!.revoke(
                    call.reference(),
                    schema.string().parse(call.input.relationshipId),
                );

                return {};
            },
        ),
        proposals: sharingMethod(
            "proposals",
            grant,
            { method: "GET", path: "/{id}/proposals" },
            (shapes) => shapes.target.extend(PageShape),
            page(Proposal.schema),
            async (call) => {
                // page the object's proposals by identifier
                const listing = sharingPage(call, "proposals");
                const proposals = await call.authorization!.proposals(
                    { object: call.reference() },
                    {
                        ...(listing.after === undefined ? {} : { after: listing.after }),
                        limit: listing.limit + 1,
                    },
                );

                return listing.result(proposals, (proposal) => proposal.id);
            },
        ),
        propose: sharingMethod(
            "propose",
            null,
            { method: "POST", path: "/{id}/proposals" },
            (shapes) => shapes.target.extend({ ...shapes.replay, ...ProposeShape }),
            Proposal.schema,
            async (call) => {
                // propose the relationship on an object outside the trash
                await requirePresent(call, "live");
                const { relationship, ...proposal } = ProposeInput.parse(call.input);

                return call.authorization!.propose({
                    ...proposal,
                    relationship: { ...relationship, object: call.reference() },
                });
            },
        ),
        accept: sharingMethod(
            "accept",
            null,
            { method: "POST", path: "/{id}/proposals/{proposalId}/accept" },
            (shapes) => shapes.target.extend({ ...shapes.replay, ...ProposalShape }),
            Relationship.schema,
            async (call) => {
                // accept only on an object outside the trash
                await requirePresent(call, "live");

                return call.authorization!.accept(
                    call.reference(),
                    schema.string().parse(call.input.proposalId),
                );
            },
        ),
        explain: defineMethod<Method>({
            kind: "explain",
            permission: null,
            mutates: false,
            target: false,
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({
                route: { method: "GET", path: "/{id}/access" },
                input: shapes.target.extend(ExplainShape),
                output: Explanation,
            }),
            effect: explain,
        }),
        decline: sharingMethod(
            "decline",
            null,
            { method: "DELETE", path: "/{id}/proposals/{proposalId}" },
            (shapes) => shapes.target.extend({ ...shapes.replay, ...ProposalShape }),
            Empty,
            async (call) => {
                // decline on any object that exists, in the trash too
                await requirePresent(call, "held");
                await call.authorization!.decline(
                    call.reference(),
                    schema.string().parse(call.input.proposalId),
                );

                return {};
            },
        ),
    }),
};

/** Declare a sharing method. */
function sharingMethod(
    kind: MethodKind,
    permission: string | null,
    route: Route,
    input: (schema: ObjectSchema) => schema.Schema,
    output: MethodProcedure["output"],
    effect: (call: Call) => Promise<unknown>,
    inverse?: (step: Step) => readonly sync.Call[] | undefined,
): Method {
    return defineMethod<Method>({
        kind,
        permission,
        mutates: permission === null,
        target: permission !== null,
        result: "value",
        isPredicted: false,
        procedure: (_name, shapes) => ({ route, input: input(shapes), output }),
        effect,
        ...(inverse === undefined ? {} : { inverse }),
    });
}

/** Explain a subject's permission on an object. */
async function explain(call: Call): Promise<Explanation> {
    // resolve access in the governing scope
    const { authorizer, snapshot } = call.authorization!;
    const { permission, subject } = schema.object(ExplainShape).parse(call.input);
    const object = call.object;
    const target = call.reference();
    const scope = authorizer.governingScope(target);
    const access = await call.authorization!.in(scope);

    // explain another subject's access only to grantors
    if (subject !== undefined) {
        const grant = object.policy.definition.grantedBy;
        if (
            grant === undefined ||
            !(await authorizer.check(snapshot, object.permission(grant), target, access)).isAllowed
        ) {
            throw new ServiceError("FORBIDDEN", {
                message: `explaining access to ${object.name} needs ${grant}`,
            });
        }
        const explained = await authorizer.resolve(snapshot, scope, {
            subjects: [subject],
            now: call.now,
            attributes: {},
        });

        return authorizer.explain(
            snapshot,
            permission === undefined ? requireReading(object) : object.permission(permission),
            target,
            explained,
        );
    }

    // explain the caller's own access to an object it may read
    if (!(await authorizer.check(snapshot, requireReading(object), target, access)).isAllowed) {
        throw new ServiceError("NOT_FOUND", { message: `no ${object.name} ${target.id}` });
    }

    return authorizer.explain(
        snapshot,
        permission === undefined ? requireReading(object) : object.permission(permission),
        target,
        access,
    );
}

/** Read the permission reading an object of a type needs. */
function requireReading(object: ObjectType) {
    if (object.reading === undefined) {
        throw new ServiceError("BAD_REQUEST", { message: `${object.name} has no objects to read` });
    }

    return object.reading;
}

/** Require the named object to exist, outside the trash when live. */
async function requirePresent(call: Call, state: "live" | "held"): Promise<void> {
    // read the object
    const table = call.object.table as Table & Record<string, Column>;
    const [row] = (await call.database
        .select()
        .from(table)
        .where(and(eq(table.id!, call.id!), call.object.inScope(call.scope)))) as Record<
        string,
        unknown
    >[];

    // refuse a missing or trashed object
    if (row === undefined) {
        throw new ServiceError("NOT_FOUND", { message: `no ${call.object.name} ${call.id}` });
    } else if (
        state === "live" &&
        row.deletionRequestedAt !== undefined &&
        row.deletionRequestedAt !== null
    ) {
        throw new ServiceError("CONFLICT", { message: `${call.object.name} is in the trash` });
    }
}

/** Read the page a sharing listing asks for, bound to its object. */
function sharingPage(call: Call, list: string) {
    return new Page(
        call.input as { cursor?: string; limit?: number },
        [call.object.name, call.scope, call.id!, list],
        schema.string(),
    );
}

/** The procedures sharing derives. */
export type ShareableProcedures<Object extends ObjectType> = {
    explain: Procedure<
        schema.Object<TargetShape<Object> & typeof ExplainShape>,
        typeof Explanation
    >;
    relationships: Procedure<
        schema.Object<TargetShape<Object> & typeof PageShape>,
        ReturnType<typeof page<typeof Relationship.schema>>
    >;
    grant: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & typeof GrantShape>,
        typeof Relationship.schema
    >;
    revoke: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & typeof RelationshipShape>,
        schema.Object<{}>
    >;
    proposals: Procedure<
        schema.Object<TargetShape<Object> & typeof PageShape>,
        ReturnType<typeof page<typeof Proposal.schema>>
    >;
    propose: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & typeof ProposeShape>,
        typeof Proposal.schema
    >;
    accept: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & typeof ProposalShape>,
        typeof Relationship.schema
    >;
    decline: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & typeof ProposalShape>,
        schema.Object<{}>
    >;
};
