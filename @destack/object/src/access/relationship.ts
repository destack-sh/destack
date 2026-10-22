import * as access from "@destack/access";
import { accessRelationship, Relationship } from "@destack/access";
import { GrantFields, readGrant, RelationshipFields } from "../trait/shareable.ts";
import { ObjectReference } from "@destack/sync";
import { schema } from "@destack/schema";
import { defineMethod } from "../method/method.ts";
import { type MethodBuilder } from "../method/record.ts";
import { role } from "./role.ts";
import { defineObject } from "../object/object.ts";
import { defineShape } from "../replica/shape.ts";
import { Execution } from "../method/execution.ts";
import { CheckInput, CheckResult, ObjectPolicy } from "../object/access.ts";

/** A relationship to create: its object, and its subject through one relation or role. */
const RelationshipInput = schema.object({ object: ObjectReference, ...GrantFields });

/** One relationship of an object, by its identifier. */
const RelationshipKey = schema.object({ object: ObjectReference, ...RelationshipFields });

/** The relationships of a scope, created and deleted under the grant rules. */
export const relationship = defineObject(
    {
        name: "relationship",
        plural: "relationships",
        mapping: ObjectPolicy.readMapping(accessRelationship),
        represents: access.relationship,
        permissions: access.relationship.definition.permissions,
        declarable: { from: "create", references: { object: "object", role, subject: "subject" } },
    },
    (_method: MethodBuilder<typeof accessRelationship>) => ({
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
            handler: (context, execution) => {
                // grant under the relation's grant rules
                const { object, ...granted } = RelationshipInput.parse(context.input);

                return Relationship.grant(
                    {
                        ...readGrant(granted),
                        object,
                    },
                    Execution.store(execution, context).authorization,
                );
            },
        }),
        delete: defineMethod<{
            kind: "custom";
            permission: "grant";
            input: typeof RelationshipKey;
            output: typeof access.Relationship.schema;
            mutates: true;
            target: "scope";
            table: typeof accessRelationship;
        }>({
            kind: "custom",
            permission: "grant",
            input: RelationshipKey,
            output: access.Relationship.schema,
            mutates: true,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, schemas) => ({
                route: { method: "DELETE", path: "/{relationshipId}" },
                input: schemas.scope.extend({ ...schemas.replay, ...RelationshipKey.shape }),
                output: access.Relationship.schema,
            }),
            handler: (context, execution) => {
                // revoke under the relation's grant rules
                const { object, relationshipId } = RelationshipKey.parse(context.input);

                return Relationship.revoke(
                    object,
                    relationshipId,
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
