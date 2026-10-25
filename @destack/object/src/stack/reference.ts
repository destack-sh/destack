import { DeclarationName } from "@destack/package";
import type { MethodInput, MethodName, ScopeField } from "../method/procedure.ts";
import type { ObjectConfiguration, ObjectType } from "../object/object.ts";
import type { Field } from "../field/field.ts";
import { schema } from "@destack/schema";
import { AccessName } from "@destack/sync";

/** The scopes of declared subjects: the stack's scope itself, or the one enclosing it. */
export const STACK_SCOPES = ["self", "enclosing"] as const;
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
        /** A record's scope: the stack's scope itself if absent, none for a global principal. */
        scope: schema.enum(STACK_SCOPES).exactOptional(),
        /** The relation of its members, for a subject set. */
        relation: AccessName.exactOptional(),
    }),
    schema.object({
        /** The declared record's type, as its collection's object. */
        kind: AccessName,
        /** The declared record's name. */
        name: DeclarationName,
        /** The relation of its members, for a subject set. */
        relation: AccessName.exactOptional(),
    }),
    schema.object({
        /** The stack's scope or the one enclosing it. */
        scope: schema.enum(STACK_SCOPES),
        /** The relation of its members. */
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
    Object extends ObjectType<infer Configuration>
        ? StackCreateInputOf<
              MethodInput<Object, "create" & MethodName<Object>>,
              "name" | StackSuppliedOf<Object>,
              ReferencesOf<Configuration>
          > &
              NameInputOf<MethodInput<Object, "create" & MethodName<Object>>> & {
                  /** The scope the record goes in, the stack's own by default. */
                  readonly scope?: StackScope;
              }
        : never;

/** A create input's name, which the declaration's name fills when absent. */
type NameInputOf<Input> = "name" extends keyof Input
    ? { readonly name?: Input["name" & keyof Input] }
    : {};

/** The create fields a declaration refers to other records by: references, parent and listed. */
type ReferencesOf<Configuration extends ObjectConfiguration> = FieldReferencesOf<
    Configuration["fields"]
> &
    (Configuration["parent"] extends "any" | undefined ? {} : { readonly parentId: ObjectType }) &
    Configuration["references"];

/** The reference fields of some fields keyed by column, by create field: a record, or a subject. */
type FieldReferencesOf<Fields> = {
    readonly [
        Name in keyof Fields as Fields[Name] extends Field<infer Configuration>
            ? Configuration["reference"] extends "identifier" | "object"
                ? Name
                : never
            : never
    ]: Fields[Name] extends Field<infer Configuration>
        ? Configuration["reference"] extends "identifier"
            ? ObjectType
            : "subject"
        : never;
};

/** The create fields the stack fills for a declaration: its identity, scope and owner. */
export type StackSuppliedOf<Object extends ObjectType> =
    Object extends ObjectType<infer Configuration>
        ?
              | "id"
              | ScopeField<Object>
              | (Configuration["owner"] extends string ? `${Configuration["owner"]}Id` : never)
              | Configuration["supplied"]
        : never;

/** A create input as a stack document writes it, without its key and its supplied fields. */
type StackCreateInputOf<Input, Omitted, References> = Omit<
    Input,
    (Omitted & string) | keyof References
> & {
    readonly [
        Key in Extract<
            Exclude<keyof References, Omitted>,
            keyof Input
        > as References[Key] extends "object"
            ? never
            : undefined extends Input[Key & keyof Input]
              ? never
              : DeclaredField<Key & string>
    ]: ReferenceInputOf<References[Key]>;
} & {
    readonly [
        Key in Extract<
            Exclude<keyof References, Omitted>,
            keyof Input
        > as References[Key] extends "object"
            ? DeclaredField<Key & string>
            : undefined extends Input[Key & keyof Input]
              ? DeclaredField<Key & string>
              : never
    ]?: ReferenceInputOf<References[Key]>;
};

/** A reference field as a declaration writes it, without its `Id` suffix. */
type DeclaredField<Name extends string> = Name extends `${infer Base}Id` ? Base : Name;

/** What a declared reference refers to: one type's records, any object, or a subject. */
export type DeclaredReference = ObjectType | "object" | "subject";
