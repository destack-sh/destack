import type * as db from "@destack/db";
import type { RowImage } from "@destack/db";
import type { Field, IdentifierOf } from "../field/field.ts";
import type { IdentityOf, InstanceOf, ObjectConfiguration, ObjectType } from "../object/object.ts";

/** What relational reads see of a served object type: its logged row, instance and relations. */
export type ModelOf<Object extends ObjectType, Objects extends ObjectType> = {
    /** The logged fields conditions and orders compare. */
    readonly row: Object extends ObjectType<infer Configuration>
        ? RowImage<Configuration["table"]>
        : never;
    /** The fields each result has. */
    readonly value: InstanceOf<Object>;
    /** The relations, each to the related type's model. */
    readonly relations: {
        readonly [Name in keyof RelationsOf<Object, Objects>]: RelationsOf<
            Object,
            Objects
        >[Name] extends One<infer Target>
            ? db.One<ModelOf<Target, Objects>>
            : RelationsOf<Object, Objects>[Name] extends Many<infer Target>
              ? db.Many<ModelOf<Target, Objects>>
              : never;
    };
};

/** A relation to at most one object of a type. */
interface One<Target extends ObjectType = ObjectType> {
    /** The related object type. */
    readonly object: Target;
    /** How many objects a row relates to. */
    readonly kind: "one";
}

/** A relation to any number of objects of a type. */
interface Many<Target extends ObjectType = ObjectType> {
    /** The related object type. */
    readonly object: Target;
    /** How many objects a row relates to. */
    readonly kind: "many";
}

/** The relations of an object type among the served types, by name, as queries join them. */
type RelationsOf<Object extends ObjectType, Objects extends ObjectType> =
    Object extends ObjectType<infer Configuration>
        ? string extends Configuration["name"]
            ? {}
            : ReferenceRelations<Configuration, Objects> &
                  ParentRelation<Object, Configuration["parent"], Objects> &
                  TreeRelations<Object, Configuration> &
                  ChildRelations<Object, Objects> &
                  JunctionRelations<Object, ChildRelations<Object, Objects>, Objects>
        : never;

/** The relations of a type's reference fields, each to one object of its type or of several. */
type ReferenceRelations<Configuration extends ObjectConfiguration, Objects> = {
    readonly [
        Key in keyof Configuration["fields"] as ReferenceName<Configuration["fields"][Key], Objects>
    ]: One<TargetOf<Configuration["fields"][Key], Objects>>;
} & {
    readonly [
        Key in keyof Configuration["fields"] as ObjectReferenceName<Configuration["fields"][Key]>
    ]: One<Extract<Objects, ObjectType>>;
};

/** The relation a served reference declares, absent for other fields. */
type ReferenceName<Declaration, Objects> =
    Declaration extends Field<infer Configuration>
        ? [Configuration["relation"]] extends [never]
            ? never
            : [TargetOf<Declaration, Objects>] extends [never]
              ? never
              : Configuration["relation"]
        : never;

/** The relation an object reference field declares, absent for other fields. */
type ObjectReferenceName<Declaration> =
    Declaration extends Field<infer Configuration>
        ? Configuration["reference"] extends "object"
            ? [Configuration["relation"]] extends [never]
                ? never
                : Configuration["relation"]
            : never
        : never;

/** The served object type a reference field's identifiers belong to. */
type TargetOf<Declaration, Objects> =
    Declaration extends Field<infer Configuration>
        ? ObjectByIdentifier<Objects, Configuration["value"]>
        : never;

/** The object types whose identifiers are a branded value, never for plain strings. */
type ObjectByIdentifier<Objects, Value> = string extends Value
    ? never
    : Objects extends ObjectType
      ? IsSame<IdentifierOf<Objects>, Value> extends true
          ? Objects
          : never
      : never;

/** The relation to a served parent type. */
type ParentRelation<Object extends ObjectType, Parent, Objects> = [Parent] extends ["self"]
    ? { readonly parent: One<Object> }
    : [Parent] extends [undefined | "any"]
      ? {}
      : [ObjectByIdentity<Objects, Parent>] extends [never]
        ? {}
        : { readonly parent: One<ObjectByIdentity<Objects, Parent>> };

/** The served object types of an identity. */
type ObjectByIdentity<Objects, Identity> =
    Objects extends ObjectType<infer Configuration>
        ? IsSame<Configuration["identity"], Identity> extends true
            ? Objects
            : never
        : never;

/** The relations of a tree to the objects below and above each object. */
type TreeRelations<Object extends ObjectType, Configuration extends ObjectConfiguration> = [
    Configuration["parent"],
] extends ["self"]
    ? { readonly descendants: Many<Object>; readonly ancestors: Many<Object> }
    : {};

/** The relations to the served types listing an object type, by their plural. */
type ChildRelations<Object extends ObjectType, Objects extends ObjectType> = {
    readonly [Plural in PluralOf<ChildOf<Object, Objects>>]: Many<
        ObjectByPlural<ChildOf<Object, Objects>, Plural>
    >;
};

/** The served types nested in, attached to, or referencing an object type through one field. */
type ChildOf<Object extends ObjectType, Objects> =
    Objects extends ObjectType<infer Configuration>
        ? IsSame<Objects, Object> extends true
            ? never
            : IsSame<Configuration["parent"], IdentityOf<Object>> extends true
              ? Objects
              : Configuration["identity"] extends AttachmentsOf<Object>
                ? Objects
                : IsSingle<ReferenceTo<Configuration["fields"], Object>> extends true
                  ? Objects
                  : never
        : never;

/** The relations through each junction child to the type it joins, by that type's plural. */
type JunctionRelations<Object extends ObjectType, Children, Objects> = {
    readonly [Plural in PluralOf<JoinedOf<Object, Children, Objects>>]: Many<
        ObjectByPlural<JoinedOf<Object, Children, Objects>, Plural>
    >;
};

/** The types a type's junction children join it to, other than itself. */
type JoinedOf<Object extends ObjectType, Children, Objects> = {
    [Near in keyof Children]: Children[Near] extends Many<infer Junction>
        ? Junction extends ObjectType<infer Configuration>
            ? Configuration["isJunction"] extends true
                ? Exclude<TargetsOf<ReferenceRelations<Configuration, Objects>>, Object>
                : never
            : never
        : never;
}[keyof Children];

/** The types some relations relate one of. */
type TargetsOf<Relations> = {
    [Name in keyof Relations]: Relations[Name] extends One<infer Target> ? Target : never;
}[keyof Relations];

/** The identities of the attachment types an object type takes. */
type AttachmentsOf<Object extends ObjectType> =
    Object extends ObjectType<infer Configuration> ? Configuration["attachments"] : never;

/** The names of the unqualified reference fields targeting an object type. */
type ReferenceTo<Fields, Object extends ObjectType> = {
    [Key in keyof Fields]: [TargetOf<Fields[Key], Object>] extends [never] ? never : Key;
}[keyof Fields];

/** The plural names of object types. */
type PluralOf<Objects> =
    Objects extends ObjectType<infer Configuration> ? Configuration["plural"] : never;

/** The object types of a plural name. */
type ObjectByPlural<Objects, Plural> =
    Objects extends ObjectType<infer Configuration>
        ? Configuration["plural"] extends Plural
            ? Objects
            : never
        : never;

/** Whether two types are the same. */
type IsSame<Left, Right> = [Left] extends [Right] ? ([Right] extends [Left] ? true : false) : false;

/** Whether a union has exactly one member. */
type IsSingle<Union> = [Union] extends [never] ? false : IsUnion<Union> extends true ? false : true;

/** Whether a type is a union of several members. */
type IsUnion<Union, Whole = Union> = Union extends unknown
    ? [Whole] extends [Union]
        ? false
        : true
    : never;
