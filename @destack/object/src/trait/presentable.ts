import { ACCENT_PRESETS, type AccentPreset } from "@destack/theme";
import { defineSchema, type JsonValue, present, schema } from "@destack/schema";
import { field, type Field } from "../field/field.ts";
import type { ObjectDefinition } from "../object/object.ts";
import type { Erasure } from "./trait.ts";
import { BucketName, type FileBucket, ObjectFile } from "../field/file.ts";

/** The fields an object is titled by when it names none, the first one it has. */
const TITLE_FIELDS = ["title", "name"] as const;

/** The longest emoji in UTF-16 code units: 15, the longest RGI sequence. */
const EMOJI_LENGTH = 15;

/** The accents an object's color takes: the theme's colorful presets, each with its swatch. */
const [FIRST_ACCENT, ...OTHER_ACCENTS] = ACCENT_PRESETS;

/** The accents an object's color takes. */
export const ACCENTS: readonly [AccentPreset, ...AccentPreset[]] = [
    present(FIRST_ACCENT, "the theme's first accent"),
    ...OTHER_ACCENTS,
];

/** A name in an icon set, such as `book-open`. */
const IconName = schema.string().regex(/^[a-z0-9]+(?:-[a-z0-9]+)*$/u);

/** An object's icon chosen from an emoji or an icon set. */
const ChosenIcon = [
    schema.object({
        /** The emoji. */
        emoji: schema.emoji().max(EMOJI_LENGTH),
    }),
    schema.object({
        /** The icon's name in the icon set. */
        icon: IconName,
    }),
] as const;

/** An object's icon: an emoji, an icon set's icon, or an image kept in the bucket its type names. */
export const Icon = defineSchema(
    schema.union([
        ...ChosenIcon,
        schema.object({
            /** The image. */
            file: ObjectFile,
        }),
    ]),
);
/** An object's icon. */
export type Icon = schema.Infer<typeof Icon>;

/** An object's cover: an image kept in the bucket its type names, and the vertical point the banner centres on. */
export const Cover = defineSchema(
    schema.object({
        /** The image. */
        file: ObjectFile,
        /** The point the banner centres on, from 0 at the top to 1 at the bottom. */
        focus: schema.number().min(0).max(1),
    }),
);
/** An object's cover. */
export type Cover = schema.Infer<typeof Cover>;

/** How an object type presents its objects, piece by piece. */
export interface PresentableOptions {
    /** The field titling each object, its `title` or `name` field when absent. */
    readonly title?: string;
    /** The field shown under the title. */
    readonly subtitle?: string;
    /** Each object's chosen icon, with images kept in a bucket, one fixed icon set name for every object, or none, chosen when absent. */
    readonly icon?: boolean | string | FileBucket;
    /** Whether each object takes one of the theme's accents, true when absent. */
    readonly color?: boolean;
    /** The bucket keeping each object's cover image, none when absent. */
    readonly cover?: FileBucket;
}

/** The presentation an object type declares: every piece at its default, or each piece set. */
export type PresentableDefinition = true | PresentableOptions;

/** The fields presentation adds: the chosen icon, the accent and the cover. */
export type PresentationFieldsOf<Presenting> = Presenting extends PresentableDefinition
    ? (Presenting extends { readonly icon: false | string }
          ? {}
          : {
                /** The object's icon. */
                readonly icon: ReturnType<typeof iconField>;
            }) &
          (Presenting extends { readonly color: false }
              ? {}
              : {
                    /** The object's accent. */
                    readonly color: ReturnType<typeof colorField>;
                }) &
          (Presenting extends { readonly cover: FileBucket }
              ? {
                    /** The object's cover image. */
                    readonly cover: ReturnType<typeof coverField>;
                }
              : {})
    : {};

/** The presentation of an object type, its pieces resolved to the fields keeping them. */
const PresentationSchema = defineSchema(
    schema.object({
        /** The field titling each object. */
        title: schema.string().min(1),
        /** The field shown under the title. */
        subtitle: schema.string().min(1).exactOptional(),
        /** The field keeping each object's icon and the bucket of uploaded ones, or the icon every object shows. */
        icon: schema
            .union([
                schema.object({
                    field: schema.string().min(1),
                    bucket: BucketName.exactOptional(),
                }),
                schema.object({ fixed: IconName }),
            ])
            .exactOptional(),
        /** The field keeping each object's accent. */
        color: schema.object({ field: schema.string().min(1) }).exactOptional(),
        /** The field keeping each object's cover and the bucket keeping it. */
        cover: schema.object({ field: schema.string().min(1), bucket: BucketName }).exactOptional(),
    }),
);
/** The presentation of an object type. */
export type Presentation = schema.Infer<typeof PresentationSchema>;

/** The presentation of an object type, and how an object's row reads through it. */
export const Presentation = Object.assign(PresentationSchema, {
    /** Read an object's title from its row, its identifier for a type presenting none or an untitled object. */
    title(
        presentation: Presentation | undefined,
        row: Readonly<Record<string, JsonValue | undefined>>,
    ): string {
        const title = presentation === undefined ? undefined : row[presentation.title];

        return typeof title === "string" && title !== "" ? title : schema.string().parse(row["id"]);
    },
});

/** Objects presented by a title, a subtitle, an icon, an accent and a cover, as pickers, mentions and titles show them. */
export const Presentable = {
    /** Add the icon, accent and cover fields a presented definition keeps, refusing a field it declares itself. */
    expand<Definition extends ObjectDefinition>(
        definition: Definition,
    ): Erasure<Definition, "fields"> {
        // keep a definition presenting nothing
        const options = Presentable.options(definition);
        if (options === undefined) {
            return definition;
        }

        // add each piece's field
        const added: Record<string, Field> = {
            ...(options.icon === undefined ||
            options.icon === false ||
            typeof options.icon === "string"
                ? {}
                : { icon: options.icon === true ? chosenIconField() : iconField() }),
            ...(options.color === false ? {} : { color: colorField() }),
            ...(options.cover === undefined ? {} : { cover: coverField() }),
        };
        const declared = definition.fields ?? {};
        const taken = Object.keys(added).find((name) => Object.hasOwn(declared, name));
        if (taken !== undefined) {
            throw new TypeError(
                `object ${definition.name} declares field ${taken}, which its presentation adds`,
            );
        }

        return { ...definition, fields: { ...declared, ...added } };
    },

    /** Resolve a definition's presentation to the fields keeping each piece, absent for one presenting nothing. */
    of(
        definition: Pick<ObjectDefinition, "name" | "presentable" | "fields">,
    ): Presentation | undefined {
        // present nothing without the trait
        const options = Presentable.options(definition);
        if (options === undefined) {
            return undefined;
        }

        // title by the named field or the first title field, refusing a type without one
        const fields = definition.fields ?? {};
        const title = options.title ?? TITLE_FIELDS.find((name) => Object.hasOwn(fields, name));
        for (const named of [title, options.subtitle]) {
            if (named !== undefined && !isText(fields[named])) {
                throw new TypeError(
                    `object ${definition.name} presents ${named}, which is no string field`,
                );
            }
        }
        const icon = options.icon;

        return PresentationSchema.parse({
            title: present(title, `a title or name field of ${definition.name}`),
            ...(options.subtitle === undefined ? {} : { subtitle: options.subtitle }),
            ...(icon === false || icon === undefined
                ? {}
                : typeof icon === "string"
                  ? { icon: { fixed: icon } }
                  : {
                        icon: {
                            field: "icon",
                            ...(icon === true ? {} : { bucket: BucketName.of(icon) }),
                        },
                    }),
            ...(options.color === false ? {} : { color: { field: "color" } }),
            ...(options.cover === undefined
                ? {}
                : { cover: { field: "cover", bucket: BucketName.of(options.cover) } }),
        });
    },

    /** Read a definition's presentation options, every piece at its default for `true`. */
    options(definition: Pick<ObjectDefinition, "presentable">): PresentableOptions | undefined {
        // present nothing without the trait
        const presentable = definition.presentable;
        if (presentable === undefined) {
            return undefined;
        }

        // choose an icon and an accent unless told otherwise
        const options = presentable === true ? {} : presentable;

        return { icon: true, color: true, ...options };
    },
};

/** Decide whether a field keeps text a title shows. */
function isText(declared: Field | undefined): boolean {
    return declared?.type === "string" || declared?.type === "text" || declared?.type === "enum";
}

/** The field keeping an object's icon: an emoji, an icon set's icon or an image. */
function iconField() {
    return field.json(Icon).optional();
}

/** The field keeping an object's icon chosen from an emoji or an icon set, without images. */
function chosenIconField() {
    return field.json<Icon>(schema.union([...ChosenIcon])).optional();
}

/** The field keeping an object's accent. */
function colorField() {
    return field.enum(ACCENTS).optional();
}

/** The field keeping an object's cover. */
function coverField() {
    return field.json(Cover).optional();
}
