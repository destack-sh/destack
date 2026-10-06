import { TABLE, uniqueIndex } from "@destack/db";
import { present, schema } from "@destack/schema";
import type { ObjectType } from "../object/object.ts";
import type { Trait } from "./trait.ts";

/** How objects project the rows of another object type people receive, one object in each recipient's home, as a read model of them. */
export interface ProjectedDefinition {
    /** The object type whose rows the objects project. */
    readonly from: ObjectType | (() => ObjectType);
    /** The subject field of the source naming each row's recipient. */
    readonly to: string;
    /** The qualified reference field naming each object's source row, unique in its scope. */
    readonly source: string;
    /** The fields kept equal to the source row's, each naming its source field, such as the source's scope. */
    readonly fields?: Readonly<Record<string, string>>;
    /** The object's own fields a change of the kept fields clears, such as its read state. */
    readonly clears?: readonly string[];
    /** The copied types naming the residents of each home and the installations keeping rows for them. */
    readonly residence: Residence;
}

/** The copied types telling a home whose rows it projects: its residents' users with their homes, and the residences in their scopes. */
export interface Residence {
    /** The users, each with the `home` field naming its home space. */
    readonly user: ObjectType;
    /** The residences in users' scopes, each with the `source` scope and the `installation` keeping rows for the user. */
    readonly residence: ObjectType;
}

/** The qualified reference of a source row, as the source field keeps it. */
export const SourceReference = schema.object({
    scope: schema.string().min(1),
    id: schema.string().min(1),
});

/** Objects projected from another type's rows: each its own object with its own state, created with its source and retracted when the source leaves. */
export const projected: Trait<ProjectedDefinition> & {
    /** Read the source type a projection names. */
    sourceOf(options: ProjectedDefinition): ObjectType;
} = {
    key: "projected",
    isDurable: true,
    options: (definition) => definition.projected,
    columns: () => ({}),
    constraints: (options, table, columns) => [
        uniqueIndex(`${table}_source`).on(
            present(columns["scope"], "the scope column"),
            present(columns[options.source], "the source column"),
        ),
    ],
    methods: () => ({}),
    validate: (options, object) => {
        // require a qualified reference to the source type, and a subject field naming its recipient
        const field = object.fields[options.source];
        const source = projected.sourceOf(options);
        if (field?.type !== "reference" || !field.qualified || !source.same(field.target?.())) {
            throw new TypeError(
                `object ${object.name} projects ${source.name} through no qualified reference field ${options.source}`,
            );
        } else if (source.fields[options.to]?.type !== "subject") {
            throw new TypeError(
                `object ${object.name} projects ${source.name} to no subject field ${options.to}`,
            );
        }

        // require the kept fields on both types, and the cleared fields on the object's own
        for (const [kept, from] of Object.entries(options.fields ?? {})) {
            if (
                object.fields[kept] === undefined ||
                !Object.hasOwn(source.table[TABLE].columns, from)
            ) {
                throw new TypeError(
                    `object ${object.name} keeps no field ${kept} from ${source.name}'s ${from}`,
                );
            }
        }
        const cleared = (options.clears ?? []).find(
            (name) =>
                object.fields[name] === undefined || Object.hasOwn(options.fields ?? {}, name),
        );
        if (cleared !== undefined) {
            throw new TypeError(`object ${object.name} clears no own field ${cleared}`);
        }

        // require the residents' homes and the residences naming their sources
        const { user, residence } = options.residence;
        const missing = [
            ...(user.fields["home"] === undefined ? [`${user.name}.home`] : []),
            ...["source", "installation"].flatMap((name) =>
                residence.fields[name] === undefined ? [`${residence.name}.${name}`] : [],
            ),
        ];
        if (missing.length > 0) {
            throw new TypeError(
                `object ${object.name} projects for residents without ${missing.join(", ")}`,
            );
        }
    },
    /** Read the source type a projection names. */
    sourceOf(options: ProjectedDefinition): ObjectType {
        return typeof options.from === "function" ? options.from() : options.from;
    },
};
