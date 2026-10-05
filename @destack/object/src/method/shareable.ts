import { Explanation, Invitation, Relationship } from "@destack/access";
import { and, eq, TABLE, type Row } from "@destack/db";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Page, page } from "@destack/service/page";
import { Call } from "./call.ts";
import { Step } from "./step.ts";
import type * as sync from "@destack/sync";
import { defineMethod, type Method, type MethodKind, type MethodConfiguration } from "./method.ts";
import {
    Empty,
    PageShape,
    type MethodProcedure,
    type ObjectSchema,
    type Route,
} from "./procedure.ts";
import { type ObjectType } from "../object/object.ts";
import type { SharingGate } from "../trait/shareable.ts";

import {
    GrantShape,
    GrantInput,
    InviteShape,
    InviteInput,
    RelationshipShape,
    ExplainShape,
    InvitationShape,
} from "../trait/shareable.ts";
/** Declare the sharing methods of an object: its relationships, grants, invitations and explanations. */
export function sharingMethods({ grant, read }: SharingGate) {
    return {
        relationships: sharingMethod(
            "relationships",
            read,
            { method: "GET", path: "/{id}/relationships" },
            (shapes) => shapes.target.extend(PageShape),
            page(Relationship.schema),
            async (call) => {
                // page the object's relationships by identifier
                const listing = sharingPage(call, "relationships");
                const authorization = call.requireAuthorization();
                const relationships = await authorization.authorizer.relationships(
                    authorization.snapshot,
                    call.reference(),
                    authorization.access,
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

                return call.requireAuthorization().grant({
                    ...GrantInput.parse(call.input),
                    object: call.reference(),
                });
            },
            (step) => {
                // revoke the granted relationship
                const granted = Call.resultId(step.result);
                const call =
                    granted === undefined
                        ? undefined
                        : Step.call(step, "revoke", {
                              ...Step.target(step),
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
                await requirePresent(call, "stored");
                await call
                    .requireAuthorization()
                    .revoke(
                        call.reference(),
                        schema.object(RelationshipShape).parse(call.input).relationshipId,
                    );

                return {};
            },
        ),
        invitations: sharingMethod(
            "invitations",
            grant,
            { method: "GET", path: "/{id}/invitations" },
            (shapes) => shapes.target.extend(PageShape),
            page(Invitation.schema),
            async (call) => {
                // page the object's invitations by identifier
                const listing = sharingPage(call, "invitations");
                const invitations = await call.requireAuthorization().invitations(
                    { object: call.reference() },
                    {
                        ...(listing.after === undefined ? {} : { after: listing.after }),
                        limit: listing.limit + 1,
                    },
                );

                return listing.result(invitations, (invitation) => invitation.id);
            },
        ),
        invite: sharingMethod(
            "invite",
            null,
            { method: "POST", path: "/{id}/invitations" },
            (shapes) => shapes.target.extend({ ...shapes.replay, ...InviteShape }),
            Invitation.schema,
            async (call) => {
                // invite to the relationship on an object outside the trash
                // TODO #Incomplete: send an invitation notification when the invitation names a contact
                await requirePresent(call, "live");
                const { relationship, ...invitation } = InviteInput.parse(call.input);

                return call.requireAuthorization().invite({
                    ...invitation,
                    relationship: { ...relationship, object: call.reference() },
                });
            },
        ),
        accept: sharingMethod(
            "accept",
            null,
            { method: "POST", path: "/{id}/invitations/{invitationId}/accept" },
            (shapes) => shapes.target.extend({ ...shapes.replay, ...InvitationShape }),
            Relationship.schema,
            async (call) => {
                // accept only on an object outside the trash
                await requirePresent(call, "live");

                return call
                    .requireAuthorization()
                    .accept(
                        call.reference(),
                        schema.object(InvitationShape).parse(call.input).invitationId,
                    );
            },
        ),
        explain: defineMethod<MethodConfiguration>({
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
            handler: explain,
        }),
        withdraw: sharingMethod(
            "withdraw",
            null,
            { method: "DELETE", path: "/{id}/invitations/{invitationId}" },
            (shapes) => shapes.target.extend({ ...shapes.replay, ...InvitationShape }),
            Empty,
            async (call) => {
                // withdraw on any object that exists, in the trash too
                await requirePresent(call, "stored");
                await call
                    .requireAuthorization()
                    .withdraw(
                        call.reference(),
                        schema.object(InvitationShape).parse(call.input).invitationId,
                    );

                return {};
            },
        ),
    };
}

/** Declare a sharing method. */
function sharingMethod(
    kind: MethodKind,
    required: string | null,
    route: Route,
    input: (schema: ObjectSchema) => schema.JsonObject,
    output: MethodProcedure["output"],
    handler: (call: Call) => Promise<unknown>,
    inverse?: (step: Step) => readonly sync.Call[] | undefined,
): Method {
    return defineMethod<MethodConfiguration>({
        kind,
        permission: required,
        mutates: required === null,
        target: required !== null,
        result: "value",
        isPredicted: false,
        procedure: (_name, shapes) => ({ route, input: input(shapes), output }),
        handler,
        ...(inverse === undefined ? {} : { inverse }),
    });
}

/** Explain a subject's permission on an object. */
async function explain(call: Call): Promise<Explanation> {
    // resolve access in the governing scope
    const { authorizer, snapshot } = call.requireAuthorization();
    const { permission: asked, subject } = schema.object(ExplainShape).parse(call.input);
    const object = call.object;
    const target = call.reference();
    const scope = authorizer.governingScope(target);
    const access = await call.requireAuthorization().in(scope);
    const explained = () =>
        asked === undefined ? requireReading(object) : object.permission(asked);

    // explain another subject's access only to grantors
    if (subject !== undefined) {
        const grant = object.policy.definition.grantedBy;
        const granting =
            grant === undefined
                ? undefined
                : await authorizer.check(snapshot, object.permission(grant), target, access);
        if (granting?.isAllowed !== true) {
            throw new ServiceError("FORBIDDEN", {
                message: `explaining access to ${object.name} needs ${grant}`,
            });
        }
        const resolved = await authorizer.resolve(snapshot, scope, {
            subjects: [subject],
            now: call.now,
            attributes: {},
        });

        return authorizer.explain(snapshot, explained(), target, resolved);
    }
    // explain the caller's own access to an object it may read
    else {
        const reading = await authorizer.check(snapshot, requireReading(object), target, access);
        if (!reading.isAllowed) {
            throw new ServiceError("NOT_FOUND", { message: `no ${object.name} ${target.id}` });
        }

        return authorizer.explain(snapshot, explained(), target, access);
    }
}

/** Read the permission reading an object of a type needs. */
function requireReading(object: ObjectType) {
    if (object.reading === undefined) {
        throw new ServiceError("BAD_REQUEST", { message: `${object.name} has no objects to read` });
    }

    return object.reading;
}

/** Require the named object to exist, outside the trash when live. */
async function requirePresent(call: Call, state: "live" | "stored"): Promise<void> {
    // read the object
    const table = call.object.table;
    const rows: readonly Row[] = await call.database
        .select()
        .from(table)
        .where(
            and(eq(table[TABLE].column("id"), call.requireId()), call.object.inScope(call.scope)),
        );
    const [row] = rows;

    // refuse a missing or trashed object
    if (row === undefined) {
        throw new ServiceError("NOT_FOUND", { message: `no ${call.object.name} ${call.id}` });
    } else if (
        state === "live" &&
        row["deletionRequestedAt"] !== undefined &&
        row["deletionRequestedAt"] !== null
    ) {
        throw new ServiceError("CONFLICT", { message: `${call.object.name} is in the trash` });
    }
}

/** Read the page a sharing listing asks for, bound to its object. */
function sharingPage(call: Call, list: string) {
    return new Page(
        call.input,
        [call.object.name, call.scope, call.requireId(), list],
        schema.string(),
    );
}
