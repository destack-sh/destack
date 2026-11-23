import type { Moment } from "@destack/db";
import type { Localization, Message } from "@destack/locale";
import { type JsonObject, schema } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import { Field } from "../field/field.ts";
import type { ObjectType } from "../object/object.ts";
import { ObjectAppearance } from "./appearance.ts";
import { ObjectValue } from "./value.ts";
import { ObjectView } from "./view.ts";

/** The title of a detail's one section when its type declares none. */
const DEFAULT_SECTION = "Details";

/** The most fields a detail's key section shows when its type names none. */
const KEY_FIELDS = 3;

/** One field of an object and its value. */
export interface ObjectField {
    /** The field's name. */
    readonly field: string;
    /** The field's label as declared, absent to show its name. */
    readonly label: Message | undefined;
    /** The field's value, by what it means. */
    readonly value: ObjectValue;
}

/** One titled group of an object's fields. */
export interface ObjectSection {
    /** The section's title. */
    readonly title: string;
    /** Its fields. */
    readonly fields: readonly ObjectField[];
}

/** The objects of one type that reference an object, as a view filtered to it. */
export interface ObjectRelated {
    /** The referencing type, by name. */
    readonly type: string;
    /** Its reference field pointing to the object. */
    readonly field: string;
    /** The view of the referencing objects. */
    readonly view: ObjectView;
}

/** One object in detail: its reference, appearance, sections and referrers. */
export interface ObjectDetail {
    /** The object. */
    readonly reference: ObjectReference;
    /** How it looks. */
    readonly appearance: ObjectAppearance;
    /** Its fields, in titled sections. */
    readonly sections: readonly ObjectSection[];
    /** The objects that reference it, one view per referencing field. */
    readonly related: readonly ObjectRelated[];
}

/** One object in detail, read from its row through its type's presentation. */
export const ObjectDetail = {
    /** Read an object's key fields as one section, its title in a locale. */
    key(
        type: ObjectType,
        row: JsonObject,
        moment: Moment,
        localization: Localization,
        key: string | undefined,
    ): ObjectDetail {
        const names = ObjectDetail.keys(type);

        return {
            ...identify(type, row, moment, localization, key),
            sections: [
                {
                    title: DEFAULT_SECTION,
                    fields: names.map((name) => ObjectDetail.field(type, row, name, localization)),
                },
            ],
            related: [],
        };
    },

    /** Read an object's fields in its type's sections, and its referrers among some types. */
    full(
        type: ObjectType,
        row: JsonObject,
        types: readonly ObjectType[],
        moment: Moment,
        localization: Localization,
        key: string | undefined,
    ): ObjectDetail {
        // take the declared sections, else one with every shown field
        const declared = type.presentation.page;
        const sections = declared?.sections ?? [
            { title: DEFAULT_SECTION, fields: ObjectDetail.shown(type) },
        ];

        // list each reference field of the given types that points to this type, or those declared
        const id = schema.string().parse(row["id"]);
        const named = declared?.related;
        const related = types
            .flatMap((other) =>
                Object.entries(other.fields).flatMap(([field, declaredField]) =>
                    declaredField.target?.().same(type) === true
                        ? [{ type: other.name, title: other.definition.title, field }]
                        : [],
                ),
            )
            .filter((each) => named === undefined || named.includes(`${each.type}.${each.field}`))
            .map((each) => ({
                type: each.type,
                field: each.field,
                view: {
                    name: ObjectView.nameOf(each.type, "by", each.field),
                    title: each.title,
                    where: { [each.field]: id },
                    layout: { kind: "table" as const },
                },
            }));

        return {
            ...identify(type, row, moment, localization, key),
            sections: sections.map((section) => ({
                title: section.title,
                fields: section.fields.map((name) =>
                    ObjectDetail.field(type, row, name, localization),
                ),
            })),
            related,
        };
    },

    /** List a type's key fields: its declared card's, else its first shown ones. */
    keys(type: ObjectType): readonly string[] {
        return type.presentation.card ?? ObjectDetail.shown(type).slice(0, KEY_FIELDS);
    },

    /** List the fields a type shows beside its title. */
    shown(type: ObjectType): string[] {
        // leave out the presentation's own fields
        const { presentation } = type;
        const presented = new Set(
            [presentation.title, presentation.icon, presentation.color, presentation.cover].flatMap(
                (piece) =>
                    piece === undefined
                        ? []
                        : typeof piece === "string"
                          ? [piece]
                          : "field" in piece
                            ? [piece.field]
                            : [],
            ),
        );

        // leave out the scope columns of polymorphic references
        const located = new Set(
            Object.values(type.fields).flatMap((field) => {
                const columns =
                    field.reference === "object" && field.relation !== undefined
                        ? Field.columnsOf(field.relation)
                        : undefined;

                return columns === undefined
                    ? []
                    : [columns.packageId, columns.type, columns.scope];
            }),
        );

        return Object.entries(type.fields)
            .filter(
                ([name, field]) =>
                    !presented.has(name) &&
                    !located.has(name) &&
                    field.type !== "text" &&
                    field.type !== "json",
            )
            .map(([name]) => name);
    },

    /** Read one field of an object. */
    field(
        type: ObjectType,
        row: JsonObject,
        name: string,
        localization: Localization,
    ): ObjectField {
        return {
            field: name,
            label: type.fields[name]?.label,
            value: ObjectValue.of(type, name, row[name], localization),
        };
    },
};

/** Read what an object is and how it looks. */
function identify(
    type: ObjectType,
    row: JsonObject,
    moment: Moment,
    localization: Localization,
    key: string | undefined,
) {
    const scope = schema.string().parse(row["scope"]);
    const id = schema.string().parse(row["id"]);

    return {
        reference: type.reference(scope, id),
        appearance: ObjectAppearance.of(type, row, moment, localization, key),
    };
}
