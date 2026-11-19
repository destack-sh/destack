import { callableTrait } from "./trait.ts";
import { IconName } from "@destack/icon";
import { DeclarationReference } from "@destack/package/declare";
import { ACCENT_PRESETS, type AccentPreset, SwatchName } from "@destack/color";
import { type JsonValue, present, schema } from "@destack/schema";
import { field, type Field } from "../field/field.ts";
import type { ObjectDefinition } from "../object/object.ts";
import type { Erasure } from "./trait.ts";
import { type FileBucket, ObjectFile } from "../field/file.ts";
import { Condition, type OpenCondition } from "@destack/db";
import { ObjectView } from "../presentation/view.ts";

/** The options a type's presentation takes. */
const PRESENTABLE_OPTIONS: ReadonlySet<string> = new Set<keyof PresentableOptions>([
    "title",
    "subtitle",
    "icon",
    "color",
    "cover",
    "views",
    "card",
    "page",
]);

/** The fields an object is titled by when it sets none, the first one it has. */
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

/** An object's icon: an emoji, an icon set's icon, or an image in the bucket its type sets. */
export const Icon = schema.union([
    ...ChosenIcon,
    schema.object({
        /** The image. */
        file: ObjectFile,
    }),
]);
/** An object's icon. */
export type Icon = schema.Infer<typeof Icon>;

/** An object's cover: an image in its type's bucket, and the banner's vertical centre. */
export const Cover = schema.object({
    /** The image. */
    file: ObjectFile,
    /** The point the banner centres on, from 0 at the top to 1 at the bottom. */
    focus: schema.number().min(0).max(1),
});
/** An object's cover. */
export type Cover = schema.Infer<typeof Cover>;

/** How an object type presents its objects, piece by piece. */
export interface PresentableOptions {
    /** The field titling each object, its `title` or `name` field when absent. */
    readonly title?: string;
    /** The field shown under the title. */
    readonly subtitle?: string;
    /** Each object's icon: chosen images in a bucket, one fixed icon set name, or none. */
    readonly icon?: boolean | string | FileBucket;
    /** Each object's accent: stored, none, or by conditions, the first met winning. */
    readonly color?:
        | boolean
        | readonly { readonly where: OpenCondition; readonly swatch: SwatchName }[];
    /** The bucket storing each object's cover image, none when absent. */
    readonly cover?: FileBucket;
    /** The views its fields and traits cannot derive, such as a waterfall of spans. */
    readonly views?: readonly (Omit<ObjectView, "where"> & { readonly where?: OpenCondition })[];
    /** The fields a card shows beside the chip. */
    readonly card?: readonly string[];
    /** The page's sections and the related objects it lists. */
    readonly page?: PresentationPage;
}

/** An object page's sections and the related objects it lists, as `type.field`. */
export interface PresentationPage {
    /** The titled sections of fields. */
    readonly sections?: readonly { readonly title: string; readonly fields: readonly string[] }[];
    /** The referencing fields whose objects the page lists, as `type.field`. */
    readonly related?: readonly string[];
}

/** The presentation an object type declares: every piece at its default, or each piece set. */
export type PresentableDefinition = true | PresentableOptions;

/** The fields presentation adds: the chosen icon, the accent and the cover. */
export type PresentationFieldsOf<Presenting> = Presenting extends PresentableDefinition
    ? (Presenting extends { readonly icon: false | string }
          ? {}
          : {
                /** The object's icon. */
                readonly icon: ReturnType<typeof Presentable.iconField>;
            }) &
          (Presenting extends { readonly color: false | readonly unknown[] }
              ? {}
              : {
                    /** The object's accent. */
                    readonly color: ReturnType<typeof Presentable.colorField>;
                }) &
          (Presenting extends { readonly cover: FileBucket }
              ? {
                    /** The object's cover image. */
                    readonly cover: ReturnType<typeof Presentable.coverField>;
                }
              : {})
    : {};

/** The presentation of an object type, its pieces resolved to the fields storing them. */
const PresentationSchema = schema.object({
    /** The field titling each object. */
    title: schema.string().min(1),
    /** The field shown under the title. */
    subtitle: schema.string().min(1).exactOptional(),
    /** The icon field and uploaded icon bucket, or the icon of every object. */
    icon: schema
        .union([
            schema.object({
                field: schema.string().min(1),
                bucket: DeclarationReference.exactOptional(),
            }),
            schema.object({ fixed: IconName }),
        ])
        .exactOptional(),
    /** Each object's accent: its field, or conditions, the first met winning. */
    color: schema
        .union([
            schema.object({ field: schema.string().min(1) }),
            schema.object({
                conditions: schema.array(
                    schema.object({ where: Condition.schema, swatch: SwatchName }),
                ),
            }),
        ])
        .exactOptional(),
    /** The field storing each object's cover, and its bucket. */
    cover: schema
        .object({ field: schema.string().min(1), bucket: DeclarationReference })
        .exactOptional(),
    /** The views the type declares beside the derived ones. */
    views: schema.array(ObjectView).exactOptional(),
    /** The fields a card shows beside the chip. */
    card: schema.array(schema.string().min(1)).exactOptional(),
    /** The page's sections and the related objects it lists. */
    page: schema
        .object({
            /** The titled sections of fields. */
            sections: schema
                .array(
                    schema.object({
                        /** The section's title. */
                        title: schema.string().min(1),
                        /** Its fields. */
                        fields: schema.array(schema.string().min(1)),
                    }),
                )
                .exactOptional(),
            /** The referencing fields whose objects the page lists, as `type.field`. */
            related: schema.array(schema.string().min(1)).exactOptional(),
        })
        .exactOptional(),
});
/** The presentation of an object type. */
export type Presentation = schema.Infer<typeof PresentationSchema>;

/** The presentation of an object type, and how an object's row reads through it. */
export const Presentation = Object.assign(PresentationSchema, {
    /** Read an object's title from its row, else its identifier. */
    title(
        presentation: Presentation | undefined,
        row: Readonly<Record<string, JsonValue | undefined>>,
    ): string {
        const title = presentation === undefined ? undefined : row[presentation.title];

        return typeof title === "string" && title !== "" ? title : schema.string().parse(row["id"]);
    },
});

/** Objects shown by title, subtitle, icon, accent and cover in pickers, mentions and titles. */
export const Presentable = {
    /** Add a presented definition's icon, accent and cover fields, refusing a declared namesake. */
    expand<Definition extends ObjectDefinition>(
        definition: Definition,
    ): Erasure<Definition, "fields"> {
        // leave a definition presenting nothing
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
                : {
                      icon:
                          options.icon === true
                              ? Presentable.chosenIconField()
                              : Presentable.iconField(),
                  }),
            ...(options.color === false || typeof options.color === "object"
                ? {}
                : { color: Presentable.colorField() }),
            ...(options.cover === undefined ? {} : { cover: Presentable.coverField() }),
        };
        const declared = definition.fields ?? {};
        const taken = Object.keys(added).find((name) => Object.hasOwn(declared, name));
        if (taken !== undefined) {
            throw new TypeError(
                `object '${definition.name}' cannot declare field '${taken}': its presentation adds it`,
            );
        }

        return { ...definition, fields: { ...declared, ...added } };
    },

    /** Resolve a definition's presentation to its fields, absent if it presents nothing. */
    of(
        definition: Pick<ObjectDefinition, "name" | "presentable" | "fields">,
    ): Presentation | undefined {
        // present nothing without the trait
        const options = Presentable.options(definition);
        if (options === undefined) {
            return undefined;
        }

        // title by the given field or the first title field, refusing a type without one
        const fields = definition.fields ?? {};
        const title = options.title ?? TITLE_FIELDS.find((name) => Object.hasOwn(fields, name));
        for (const named of [title, options.subtitle]) {
            if (named !== undefined && !Presentable.isText(fields[named])) {
                throw new TypeError(
                    `object '${definition.name}' presents '${named}', which is not a string field`,
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
                            ...(icon === true ? {} : { bucket: DeclarationReference.of(icon) }),
                        },
                    }),
            ...(options.color === false
                ? {}
                : typeof options.color === "object"
                  ? {
                        color: {
                            conditions: options.color.map((each) => ({
                                where: Condition.json(each.where),
                                swatch: each.swatch,
                            })),
                        },
                    }
                  : { color: { field: "color" } }),
            ...(options.cover === undefined
                ? {}
                : { cover: { field: "cover", bucket: DeclarationReference.of(options.cover) } }),
            ...(options.views === undefined
                ? {}
                : {
                      views: options.views.map((view) =>
                          view.where === undefined
                              ? view
                              : { ...view, where: Condition.json(view.where) },
                      ),
                  }),
            ...(options.card === undefined ? {} : { card: options.card }),
            ...(options.page === undefined ? {} : { page: options.page }),
        });
    },

    /** Read a definition's presentation options, every piece at its default for `true`. */
    options(definition: Pick<ObjectDefinition, "presentable">): PresentableOptions | undefined {
        // present nothing without the trait
        const presentable = definition.presentable;
        if (presentable === undefined) {
            return undefined;
        }

        // refuse an option presentation does not know, such as a misspelt one
        const options = presentable === true ? {} : presentable;
        const unknown = Object.keys(options).find((key) => !PRESENTABLE_OPTIONS.has(key));
        if (unknown !== undefined) {
            throw new TypeError(
                `presentable option '${unknown}' is invalid: presentation knows no such option`,
            );
        }

        // choose an icon and an accent unless told otherwise
        return { icon: true, color: true, ...options };
    },

    /** Decide whether a field stores text a title shows. */
    isText(declared: Field | undefined): boolean {
        return (
            declared?.type === "string" || declared?.type === "text" || declared?.type === "enum"
        );
    },

    /** The field storing an object's icon: an emoji, an icon set's icon or an image. */
    iconField() {
        return field.json(Icon).optional();
    },

    /** The field storing an object's icon chosen from an emoji or an icon set, without images. */
    chosenIconField() {
        return field.json<Icon>(schema.union([...ChosenIcon])).optional();
    },

    /** The field storing an object's accent. */
    colorField() {
        return field.enum(ACCENTS).optional();
    },

    /** The field storing an object's cover. */
    coverField() {
        return field.json(Cover).optional();
    },
};

/** Show objects with a title, an icon, a cover and an accent. */
export const presentable = callableTrait("presentable", {}, true);
