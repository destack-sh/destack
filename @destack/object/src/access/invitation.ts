import * as access from "@destack/access";
import { accessInvitation, Invitation, INVITATION_PAGE } from "@destack/access";
import { InvitationShape, InviteInput, InviteShape } from "../trait/shareable.ts";
import { Page } from "../method/page.ts";
import { ListPage, PageShape } from "../method/procedure.ts";
import { ObjectReference } from "@destack/sync";
import { schema } from "@destack/schema";
import { defineMethod } from "../method/method.ts";
import { type MethodBuilder } from "../method/record.ts";
import { defineObject } from "../object/object.ts";
import { Execution } from "../method/execution.ts";
import { ObjectPolicy } from "../object/access.ts";

/** An invitation to create: its object, the relationship it offers, and why. */
const InvitationInput = schema.object({ object: ObjectReference, ...InviteShape });

/** One invitation to an object, by its identifier. */
const InvitationKey = schema.object({ object: ObjectReference, ...InvitationShape });

/** A page of an object's pending invitations, at most access's page. */
const InvitationListing = schema.object({
    object: ObjectReference,
    ...PageShape,
    limit: schema.number().int().min(1).max(INVITATION_PAGE).exactOptional(),
});

/** The invitations of a scope, answered through sharing methods. */
export const invitation = defineObject(
    {
        name: "invitation",
        plural: "invitations",
        mapping: ObjectPolicy.readMapping(accessInvitation),
        represents: access.invitation,
        permissions: access.invitation.definition.permissions,
    },
    (_method: MethodBuilder<typeof accessInvitation>) => ({
        create: defineMethod<{
            kind: "custom";
            permission: "grant";
            input: typeof InvitationInput;
            output: typeof access.Invitation.schema;
            mutates: true;
            target: "scope";
            table: typeof accessInvitation;
        }>({
            kind: "custom",
            permission: "grant",
            input: InvitationInput,
            output: access.Invitation.schema,
            mutates: true,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({
                route: { method: "POST", path: "" },
                input: shapes.scope.extend({ ...shapes.replay, ...InvitationInput.shape }),
                output: access.Invitation.schema,
            }),
            handler: (context, execution) => {
                // invite to the relationship on the object
                const { object, ...invited } = InvitationInput.parse(context.input);
                const { relationship: offered, ...details } = InviteInput.parse(invited);

                return Invitation.create(
                    {
                        ...details,
                        relationship: { ...offered, object },
                    },
                    Execution.store(execution, context).authorization,
                );
            },
        }),
        accept: defineMethod<{
            kind: "custom";
            permission: "read";
            input: typeof InvitationKey;
            output: typeof access.Relationship.schema;
            mutates: true;
            target: "scope";
            table: typeof accessInvitation;
        }>({
            kind: "custom",
            permission: "read",
            input: InvitationKey,
            output: access.Relationship.schema,
            mutates: true,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({
                route: { method: "POST", path: "/{invitationId}/accept" },
                input: shapes.scope.extend({ ...shapes.replay, ...InvitationKey.shape }),
                output: access.Relationship.schema,
            }),
            handler: (context, execution) => {
                // accept as the caller
                const { object, invitationId } = InvitationKey.parse(context.input);

                return Invitation.accept(
                    object,
                    invitationId,
                    Execution.store(execution, context).authorization,
                );
            },
        }),
        delete: defineMethod<{
            kind: "custom";
            permission: "read";
            input: typeof InvitationKey;
            output: typeof access.Invitation.schema;
            mutates: true;
            target: "scope";
            table: typeof accessInvitation;
        }>({
            kind: "custom",
            permission: "read",
            input: InvitationKey,
            output: access.Invitation.schema,
            mutates: true,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({
                route: { method: "DELETE", path: "/{invitationId}" },
                input: shapes.scope.extend({ ...shapes.replay, ...InvitationKey.shape }),
                output: access.Invitation.schema,
            }),
            handler: (context, execution) => {
                // withdraw as its inviter, its addressee or a granter
                const { object, invitationId } = InvitationKey.parse(context.input);

                return Invitation.withdraw(
                    object,
                    invitationId,
                    Execution.store(execution, context).authorization,
                );
            },
        }),
        list: defineMethod<{
            kind: "custom";
            permission: "grant";
            input: typeof InvitationListing;
            output: ReturnType<typeof ListPage.schema<typeof access.Invitation.schema>>;
            mutates: false;
            target: "scope";
            table: typeof accessInvitation;
        }>({
            kind: "custom",
            permission: "grant",
            input: InvitationListing,
            output: ListPage.schema(access.Invitation.schema),
            mutates: false,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({
                route: { method: "GET", path: "" },
                input: shapes.scope.extend(InvitationListing.shape),
                output: ListPage.schema(access.Invitation.schema),
            }),
            handler: async (context, execution) => {
                // page the object's pending invitations by identifier
                const { object, ...paging } = InvitationListing.parse(context.input);
                const listing = new Page(
                    paging,
                    [context.object.name, context.scope, object.type, object.id],
                    schema.string(),
                );
                const { authorization } = Execution.store(execution, context);
                const invitations = await Invitation.list(
                    { object },
                    {
                        ...(listing.after === undefined ? {} : { after: listing.after }),
                        limit: listing.limit,
                    },
                    authorization,
                );

                return listing.result(invitations, (each) => each.id);
            },
        }),
    }),
);
