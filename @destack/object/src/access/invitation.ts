import * as access from "@destack/access";
import { accessInvitation, Invitation, INVITATION_PAGE } from "@destack/access";
import { InviteInput, InviteFields } from "../trait/shareable.ts";
import { Page } from "../method/page.ts";
import { Empty, ListPage, PageFields } from "../method/procedure.ts";
import { ObjectReference } from "@destack/sync";
import { schema } from "@destack/schema";
import { defineMethod } from "../method/method.ts";
import { type MethodBuilder } from "../method/record.ts";
import { defineObject } from "../object/object.ts";
import { Execution } from "../method/execution.ts";
import { ObjectPolicy } from "../object/access.ts";
import { argument, plural, t } from "@destack/locale";

/** An invitation to create: its object, the relationship it offers, and why. */
const InvitationInput = schema.object({ object: ObjectReference, ...InviteFields });

/** A page of an object's pending invitations, at most access's page. */
const InvitationListing = schema.object({
    object: ObjectReference,
    ...PageFields,
    limit: schema.number().int().min(1).max(INVITATION_PAGE).exactOptional(),
});

/** The invitations to a scope's objects: created, listed, accepted and withdrawn by identifier. */
export const invitation = defineObject(
    {
        name: "invitation",
        plural: "invitations",
        title: t`${plural(argument("count"), { one: "Invitation", other: "Invitations" })}`,
        mapping: ObjectPolicy.readMapping(accessInvitation),
        access: {
            represents: access.invitation,
            permissions: access.invitation.definition.permissions,
        },
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
            title: t`Create`,
            kind: "custom",
            permission: "grant",
            input: InvitationInput,
            output: access.Invitation.schema,
            mutates: true,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, schemas) => ({
                route: { method: "POST", path: "" },
                input: schemas.scope.extend({ ...schemas.replay, ...InvitationInput.shape }),
                output: access.Invitation.schema,
            }),
            handler: async (context, execution) => {
                // invite to the relationship on the object, in the scope it would live in
                const { object, ...invited } = InvitationInput.parse(context.input);
                const { relationship: offered, ...details } = InviteInput.parse(invited);
                const relationship = { ...offered, object };
                const store = Execution.store(execution, context);
                await ObjectPolicy.requireScope(context, relationship, store);
                await ObjectPolicy.requireObject(object, { deleted: "exclude" }, store);

                return Invitation.create({ ...details, relationship }, store.authorization);
            },
        }),
        accept: defineMethod<{
            kind: "custom";
            permission: "read";
            input: typeof Empty;
            output: typeof access.Relationship.schema;
            mutates: true;
            target: "object";
            table: typeof accessInvitation;
        }>({
            title: t`Accept`,
            kind: "custom",
            permission: "read",
            input: Empty,
            output: access.Relationship.schema,
            mutates: true,
            target: "object",
            result: "value",
            isPredicted: false,
            procedure: (_name, schemas) => ({
                route: { method: "POST", path: "/{id}/accept" },
                input: schemas.target.extend(schemas.replay),
                output: access.Relationship.schema,
            }),
            handler: async (context, execution) => {
                // accept as the caller, on an object outside the trash
                const store = Execution.store(execution, context);
                const { relationship } = await Invitation.read(
                    store.transaction,
                    context.target.id,
                );
                await ObjectPolicy.requireObject(
                    relationship.object,
                    { deleted: "exclude" },
                    store,
                );

                return Invitation.accept(context.target.id, store.authorization);
            },
        }),
        delete: defineMethod<{
            kind: "custom";
            permission: "read";
            input: typeof Empty;
            output: typeof access.Invitation.schema;
            mutates: true;
            target: "object";
            table: typeof accessInvitation;
        }>({
            title: t`Delete`,
            kind: "custom",
            permission: "read",
            input: Empty,
            output: access.Invitation.schema,
            mutates: true,
            target: "object",
            result: "value",
            isPredicted: false,
            procedure: (_name, schemas) => ({
                route: { method: "DELETE", path: "/{id}" },
                input: schemas.target.extend(schemas.replay),
                output: access.Invitation.schema,
            }),
            handler: async (context, execution) => {
                // withdraw as its inviter, its addressee or a granter, on any object that exists
                const store = Execution.store(execution, context);
                const { relationship } = await Invitation.read(
                    store.transaction,
                    context.target.id,
                );
                await ObjectPolicy.requireObject(
                    relationship.object,
                    { deleted: "include" },
                    store,
                );

                return Invitation.withdraw(context.target.id, store.authorization);
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
            title: t`List`,
            kind: "custom",
            permission: "grant",
            input: InvitationListing,
            output: ListPage.schema(access.Invitation.schema),
            mutates: false,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, schemas) => ({
                route: { method: "GET", path: "" },
                input: schemas.scope.extend(InvitationListing.shape),
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
