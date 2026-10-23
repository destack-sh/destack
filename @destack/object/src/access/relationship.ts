import * as access from "@destack/access";
import { accessRelationship, Relationship } from "@destack/access";
import { GrantFields, readGrant } from "../trait/shareable.ts";
import { ObjectReference } from "@destack/sync";
import { schema } from "@destack/schema";
import { defineMethod } from "../method/method.ts";
import { Empty } from "../method/procedure.ts";
import { type MethodBuilder } from "../method/record.ts";
import { role } from "./role.ts";
import { defineObject } from "../object/object.ts";
import { defineShape } from "../replica/shape.ts";
import { Execution } from "../method/execution.ts";
import { CheckInput, CheckResult, ObjectPolicy } from "../object/access.ts";

/** A relationship to create: its object, and its subject through one relation or role. */
const RelationshipInput = schema.object({ object: ObjectReference, ...GrantFields });

/** The relationships of a scope, created and deleted under the grant rules. */
export const relationship = defineObject(
    {
        name: "relationship",
        plural: "relationships",
        mapping: ObjectPolicy.readMapping(accessRelationship),
        represents: access.relationship,
        permissions: access.relationship.definition.permissions,
        references: { object: "object", role, subject: "subject" },
    },
    (method: MethodBuilder<typeof accessRelationship>) => ({
        list: method.list("read"),
        check: defineMethod<{
            kind: "custom";
            permission: "read";
            input: typeof CheckInput;
            output: typeof CheckResult;
            mutates: false;
            target: "scope";
            table: typeof accessRelationship;
        }>({
            kind: "custom",
            permission: "read",
            input: CheckInput,
            output: CheckResult,
            mutates: false,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, schemas) => ({
                route: { method: "POST", path: "/check" },
                input: schemas.scope.extend(CheckInput.shape),
                output: CheckResult,
            }),
            handler: (context, execution) =>
                ObjectPolicy.check(context, Execution.store(execution, context)),
        }),
        get: defineMethod<{
            kind: "custom";
            permission: "read";
            input: typeof Empty;
            output: typeof access.Relationship.schema;
            mutates: false;
            target: "object";
            table: typeof accessRelationship;
        }>({
            kind: "custom",
            permission: "read",
            input: Empty,
            output: access.Relationship.schema,
            mutates: false,
            target: "object",
            result: "value",
            isPredicted: false,
            procedure: (_name, schemas) => ({
                route: { method: "GET", path: "/{id}" },
                input: schemas.target,
                output: access.Relationship.schema,
            }),
            handler: (context) => {
                // read the relationship as it was granted
                return Promise.resolve(Relationship.decode(context.target));
            },
        }),
        create: defineMethod<{
            kind: "custom";
            permission: "grant";
            input: typeof RelationshipInput;
            output: typeof access.Relationship.schema;
            mutates: true;
            target: "scope";
            table: typeof accessRelationship;
        }>({
            kind: "custom",
            permission: "grant",
            input: RelationshipInput,
            output: access.Relationship.schema,
            mutates: true,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, schemas) => ({
                route: { method: "POST", path: "" },
                input: schemas.scope.extend({ ...schemas.replay, ...RelationshipInput.shape }),
                output: access.Relationship.schema,
            }),
            handler: async (context, execution) => {
                // grant in the scope the relationship lives in, under the relation's grant rules
                const { object, ...granted } = RelationshipInput.parse(context.input);
                const request = { ...readGrant(granted), object };
                const store = Execution.store(execution, context);
                await ObjectPolicy.requireScope(context, request, store);

                return Relationship.grant(request, store.authorization);
            },
        }),
        delete: defineMethod<{
            kind: "custom";
            permission: "read";
            input: typeof Empty;
            output: typeof access.Relationship.schema;
            mutates: true;
            target: "object";
            table: typeof accessRelationship;
        }>({
            kind: "custom",
            permission: "read",
            input: Empty,
            output: access.Relationship.schema,
            mutates: true,
            target: "object",
            result: "value",
            isPredicted: false,
            procedure: (_name, schemas) => ({
                route: { method: "DELETE", path: "/{id}" },
                input: schemas.target.extend(schemas.replay),
                output: access.Relationship.schema,
            }),
            handler: (context, execution) => {
                // revoke a relationship the caller sees, under its relation's grant rules
                const { target } = context;

                return Relationship.revoke(
                    ObjectReference.at("object", target),
                    target.id,
                    Execution.store(execution, context).authorization,
                );
            },
        }),
    }),
);

/** One subject's relationships in every scope. */
export const relationshipShape = defineShape({
    name: "relationship",
    parameters: schema.object({
        /** The subject's type, such as user. */
        subjectType: schema.string().min(1),
        /** The subject's identifier. */
        subjectId: schema.string().min(1),
    }),
    objects: [relationship],
    queries: ({ subjectType, subjectId }) => [
        relationship.query({ scopes: "every", where: { subjectType, subjectId } }),
    ],
});
