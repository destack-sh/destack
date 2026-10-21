import { DeclarationName } from "@destack/package";
import type { MethodInput, MethodName, ScopeField } from "../method/procedure.ts";
import type { ObjectType } from "../object/object.ts";
import { schema } from "@destack/schema";
import { AccessName } from "@destack/sync";

/** The scopes of declared subjects: the stack's own, or the one enclosing it. */
export const STACK_SCOPES = ["own", "enclosing"] as const;
/** The scope of a declared subject. */
export type StackScope = (typeof STACK_SCOPES)[number];

/** The references of records a stack declares: by name, or an existing one by identifier. */
export const StackReference = {
    /** Build the references of one kind's records. */
    of<const Kind extends string>(kind: Kind) {
        return schema.union([
            DeclarationName,
            schema.object({
                /** The existing record. */
                id: schema.identifier(kind),
            }),
        ]);
    },
};

/** An object the stack declares, or an existing one by its identifier. */
export const StackObjectReference = schema.union([
    schema.object({
        /** The object type, as its collection's object. */
        kind: AccessName,
        /** The stable key within that collection. */
        name: DeclarationName,
    }),
    schema.object({
        /** The persistent identifier with the object type as prefix. */
        id: schema.anyIdentifier(),
    }),
]);
/** An object the stack declares, or an existing one by its identifier. */
export type StackObjectReference = schema.Infer<typeof StackObjectReference>;

/** A subject a declaration relates: by identifier, as declared, as a scope, or anyone. */
export const StackSubject = schema.union([
    schema.object({
        /** The principal or record, its type read from the identifier's prefix. */
        id: schema.anyIdentifier(),
        /** The scope of a record, the stack's own when absent, none for a global principal. */
        scope: schema.enum(STACK_SCOPES).exactOptional(),
        /** The relation its members hold, for a subject set. */
        relation: AccessName.exactOptional(),
    }),
    schema.object({
        /** The declared record's type, as its collection's object. */
        kind: AccessName,
        /** The declared record's name. */
        name: DeclarationName,
        /** The relation its members hold, for a subject set. */
        relation: AccessName.exactOptional(),
    }),
    schema.object({
        /** The stack's scope or the one enclosing it. */
        scope: schema.enum(STACK_SCOPES),
        /** The relation its members hold. */
        relation: AccessName,
    }),
    schema.object({
        /** Every caller, signed in or anonymous. */
        anyone: schema.literal(true),
    }),
]);
/** A subject a declaration relates. */
export type StackSubject = schema.Infer<typeof StackSubject>;

/** How a stack document writes a reference field: an object, a subject, or one type's record. */
type ReferenceInputOf<Reference> = Reference extends "object"
    ? StackObjectReference
    : Reference extends "subject"
      ? StackSubject
      : DeclarationName | { readonly id: string };

/** A declaration as a stack document writes it: its create input, references by name. */
export type StackInputOf<Object extends ObjectType> =
    DeclarableOf<Object> extends {
        readonly from: "create";
    }
        ? StackCreateInputOf<
              MethodInput<Object, "create" & MethodName<Object>>,
              | (DeclarableOf<Object> extends { readonly key: infer Key } ? Key : never)
              | StackSuppliedOf<Object>,
              DeclarableOf<Object> extends { readonly references: infer References }
                  ? References
                  : {}
          >
        : schema.Input<NonNullable<Object["lifecycle"]["declarationSchema"]>>;

/** The create fields the stack fills for a declaration, unless a reference resolves them. */
export type StackSuppliedOf<Object extends ObjectType> =
    Object extends ObjectType<infer Configuration>
        ? Exclude<
              | "id"
              | ScopeField<Object>
              | (Configuration["owner"] extends string ? `${Configuration["owner"]}Id` : never)
              | (Configuration["parent"] extends "any" | undefined ? never : "parentId"),
              keyof (DeclarableOf<Object> extends { readonly references: infer References }
                  ? References
                  : {})
          >
        : never;

/** A create input as a stack document writes it, without its key and its supplied fields. */
type StackCreateInputOf<Input, Omitted, References> = Omit<
    Input,
    (Omitted & string) | keyof References
> & {
    readonly [
        Field in keyof References as References[Field] extends "object"
            ? never
            : undefined extends Input[Field & keyof Input]
              ? never
              : DeclaredField<Field & string>
    ]: ReferenceInputOf<References[Field]>;
} & {
    readonly [
        Field in keyof References as References[Field] extends "object"
            ? DeclaredField<Field & string>
            : undefined extends Input[Field & keyof Input]
              ? DeclaredField<Field & string>
              : never
    ]?: ReferenceInputOf<References[Field]>;
};

/** A reference field as a declaration writes it, without its `Id` suffix. */
type DeclaredField<Field extends string> = Field extends `${infer Base}Id` ? Base : Field;

/** How stacks declare a type's objects, as its definition writes it. */
export type DeclarableOf<Object extends ObjectType> =
    Object extends ObjectType<infer Configuration> ? Configuration["declarable"] : undefined;
