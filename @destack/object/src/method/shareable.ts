import type { InputObject } from "./input.ts";
import { AccessContext, Caller, Explanation, Invitation, Relationship } from "@destack/access";
import { type Row } from "@destack/db";
import { schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { Page } from "./page.ts";
import { MethodContext } from "./context.ts";
import { Step } from "./step.ts";
import type * as sync from "@destack/sync";
import { defineMethod, Method, type MethodKind } from "./method.ts";
import {
    Empty,
    PageShape,
    type MethodProcedure,
    type ObjectSchema,
    type Route,
    ListPage,
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
import { Execution, type StoreExecution } from "./execution.ts";

/** The signature of a sharing method, acting on one object by its identifier. */
type SharingConfiguration = {
    kind: MethodKind;
    permission: string;
    mutates: boolean;
    target: "object" | "id";
};

/** The context of a sharing method. */
type SharingContext = MethodContext<SharingConfiguration>;
/** The shareable methods' handlers: sharing an object and paging its relationships. */
export const ShareableMethod = {
    /** Declare an object's sharing methods: relationships, grants, invitations and explanations. */
    methods({ grant, read }: SharingGate) {
        return {
            relationships: ShareableMethod.sharingMethod(
                "relationships",
                { permission: read, mutates: false },
                { method: "GET", path: "/{id}/relationships" },
                (shapes) => shapes.target.extend(PageShape),
                ListPage.schema(Relationship.schema),
                async (call, { authorization }) => {
                    // page the object's relationships by identifier
                    const listing = ShareableMethod.sharingPage(call, "relationships");
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
            grant: ShareableMethod.sharingMethod(
                "grant",
                { permission: grant, mutates: true },
                { method: "POST", path: "/{id}/relationships" },
                (shapes) => shapes.target.extend({ ...shapes.replay, ...GrantShape }),
                Relationship.schema,
                async (call, { authorization }) => {
                    // grant only on an object outside the trash
                    await ShareableMethod.requirePresent(call, "live");

                    return Relationship.grant(
                        {
                            ...GrantInput.parse(call.input),
                            object: call.reference(),
                        },
                        authorization,
                    );
                },
                (step) => {
                    // revoke the granted relationship
                    const granted = Method.resultId(step.result);
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
            revoke: ShareableMethod.sharingMethod(
                "revoke",
                { permission: grant, mutates: true },
                { method: "DELETE", path: "/{id}/relationships/{relationshipId}" },
                (shapes) => shapes.target.extend({ ...shapes.replay, ...RelationshipShape }),
                Empty,
                async (call, { authorization }) => {
                    // revoke on any object that exists, in the trash too
                    await ShareableMethod.requirePresent(call, "stored");
                    await Relationship.revoke(
                        call.reference(),
                        schema.object(RelationshipShape).parse(call.input).relationshipId,
                        authorization,
                    );

                    return {};
                },
            ),
            invitations: ShareableMethod.sharingMethod(
                "invitations",
                { permission: grant, mutates: false },
                { method: "GET", path: "/{id}/invitations" },
                (shapes) => shapes.target.extend(PageShape),
                ListPage.schema(Invitation.schema),
                async (call, { authorization }) => {
                    // page the object's invitations by identifier
                    const listing = ShareableMethod.sharingPage(call, "invitations");
                    const invitations = await Invitation.list(
                        { object: call.reference() },
                        {
                            ...(listing.after === undefined ? {} : { after: listing.after }),
                            limit: listing.limit,
                        },
                        authorization,
                    );

                    return listing.result(invitations, (invitation) => invitation.id);
                },
            ),
            invite: ShareableMethod.sharingMethod(
                "invite",
                { permission: grant, mutates: true },
                { method: "POST", path: "/{id}/invitations" },
                (shapes) => shapes.target.extend({ ...shapes.replay, ...InviteShape }),
                Invitation.schema,
                async (call, { authorization }) => {
                    // invite to the relationship on an object outside the trash
                    await ShareableMethod.requirePresent(call, "live");
                    const { relationship, ...invitation } = InviteInput.parse(call.input);

                    return Invitation.create(
                        {
                            ...invitation,
                            relationship: { ...relationship, object: call.reference() },
                        },
                        authorization,
                    );
                },
            ),
            accept: ShareableMethod.sharingMethod(
                "accept",
                { permission: read, mutates: true },
                { method: "POST", path: "/{id}/invitations/{invitationId}/accept" },
                (shapes) => shapes.target.extend({ ...shapes.replay, ...InvitationShape }),
                Relationship.schema,
                async (call, { authorization }) => {
                    // accept only on an object outside the trash
                    await ShareableMethod.requirePresent(call, "live");

                    return Invitation.accept(
                        call.reference(),
                        schema.object(InvitationShape).parse(call.input).invitationId,
                        authorization,
                    );
                },
            ),
            explain: defineMethod<SharingConfiguration>({
                kind: "explain",
                permission: read,
                mutates: false,
                target: "id",
                result: "value",
                isPredicted: false,
                procedure: (_name, shapes) => ({
                    route: { method: "GET", path: "/{id}/access" },
                    input: shapes.target.extend(ExplainShape),
                    output: Explanation,
                }),
                handler: (context, execution) =>
                    ShareableMethod.explain(context, Execution.store(execution, context)),
            }),
            withdraw: ShareableMethod.sharingMethod(
                "withdraw",
                { permission: grant, mutates: true },
                { method: "DELETE", path: "/{id}/invitations/{invitationId}" },
                (shapes) => shapes.target.extend({ ...shapes.replay, ...InvitationShape }),
                Empty,
                async (call, { authorization }) => {
                    // withdraw on any object that exists, in the trash too
                    await ShareableMethod.requirePresent(call, "stored");
                    await Invitation.withdraw(
                        call.reference(),
                        schema.object(InvitationShape).parse(call.input).invitationId,
                        authorization,
                    );

                    return {};
                },
            ),
        };
    },

    /** Declare a sharing method: a listing loads its target, a change defers to its relation. */
    sharingMethod(
        kind: MethodKind,
        access: { readonly permission: string; readonly mutates: boolean },
        route: Route,
        input: (schema: ObjectSchema) => InputObject,
        output: MethodProcedure["output"],
        handler: (context: SharingContext, execution: StoreExecution) => Promise<unknown>,
        inverse?: (step: Step) => readonly sync.Call[] | undefined,
    ): Method {
        return defineMethod<SharingConfiguration>({
            kind,
            permission: access.permission,
            mutates: access.mutates,
            target: access.mutates ? "id" : "object",
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({ route, input: input(shapes), output }),
            handler: (context, execution) => handler(context, Execution.store(execution, context)),
            ...(inverse === undefined ? {} : { inverse }),
        });
    },

    /** Explain a subject's permission on an object. */
    async explain(
        context: SharingContext,
        { authorization }: StoreExecution,
    ): Promise<Explanation> {
        // resolve access in the governing scope
        const { authorizer, snapshot } = authorization;
        const { permission: asked, subject } = schema.object(ExplainShape).parse(context.input);
        const object = context.object;
        const target = context.reference();
        const scope = authorizer.governingScope(target);
        const access = await authorization.in(scope);
        const explained = () =>
            asked === undefined ? ShareableMethod.requireReading(object) : object.permission(asked);

        // explain another subject's access only to grantors
        if (subject !== undefined) {
            const grant = object.policy.definition.grantedBy;
            const granting =
                grant === undefined
                    ? undefined
                    : await authorizer.check(snapshot, object.permission(grant), target, access);
            if (granting?.isAllowed !== true) {
                throw new CallError("FORBIDDEN", {
                    message: `explaining access to ${object.name} needs ${grant}`,
                });
            }
            const resolved = await authorizer.resolve(
                snapshot,
                scope,
                AccessContext.of(Caller.of(subject), context.now),
            );

            return authorizer.explain(snapshot, explained(), target, resolved);
        }
        // explain the caller's own access to an object it may read
        else {
            const reading = await authorizer.check(
                snapshot,
                ShareableMethod.requireReading(object),
                target,
                access,
            );
            if (!reading.isAllowed) {
                throw new CallError("NOT_FOUND", { message: `no ${object.name} ${target.id}` });
            }

            return authorizer.explain(snapshot, explained(), target, access);
        }
    },

    /** Read the permission reading an object of a type needs. */
    requireReading(object: ObjectType) {
        if (object.reading === undefined) {
            throw new CallError("BAD_REQUEST", {
                message: `${object.name} has no objects to read`,
            });
        }

        return object.reading;
    },

    /** Require the given object to exist, outside the trash when live. */
    async requirePresent(context: SharingContext, state: "live" | "stored"): Promise<void> {
        // read the object
        const [row]: readonly Row[] = await context
            .read(context.object.table)
            .findMany({ where: { id: context.id, scope: context.scope } });

        // refuse a missing or trashed object
        if (row === undefined) {
            throw new CallError("NOT_FOUND", {
                message: `no ${context.object.name} ${context.id}`,
            });
        } else if (
            state === "live" &&
            row["deletedAt"] !== undefined &&
            row["deletedAt"] !== null
        ) {
            throw new CallError("CONFLICT", { message: `${context.object.name} is in the trash` });
        }
    },

    /** Read the page a sharing listing asks for, bound to its object. */
    sharingPage(context: SharingContext, list: string) {
        return new Page(
            context.input,
            [context.object.name, context.scope, context.id, list],
            schema.string(),
        );
    },
};
