import { TABLE, uniqueIndex } from "@destack/db";
import { present, schema } from "@destack/schema";
import type { ObjectType } from "../object/object.ts";
import type { Trait } from "./trait.ts";

/** How objects project another type's rows people receive into each recipient's home. */
export interface ProjectedDefinition {
    /** The object type whose rows the objects project. */
    readonly from: ObjectType | (() => ObjectType);
    /** The subject field of the source holding each row's recipient. */
    readonly to: string;
    /** The qualified reference field holding each object's source row, unique in its scope. */
    readonly source: string;
    /** The fields kept equal to the source row's, mapped to source fields, such as its scope. */
    readonly fields?: Readonly<Record<string, string>>;
    /** The object's own fields a change of the kept fields clears, such as its read state. */
    readonly clears?: readonly string[];
    /** The copied types of each home's residents and the installations keeping their rows. */
    readonly residence: Residence;
}

/** The copied types telling a home whose rows it projects. */
export interface Residence {
    /** The accounts, each personal one in its user's scope, with a home below it in the chain. */
    readonly account: ObjectType;
    /** The residences in users' scopes, each with `source`, `installation` and `service`. */
    readonly residence: ObjectType;
}

/** The qualified reference of a source row, as the source field keeps it. */
export const SourceReference = schema.object({
    scope: schema.string().min(1),
    id: schema.string().min(1),
});

/** Objects projected from another type's rows, each with its own state. */
export const projected: Trait<ProjectedDefinition> & {
    /** Read the source type a projection refers to. */
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
        // require a qualified reference to the source type, and a subject field holding its recipient
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

        // require the accounts telling personal ones apart and the residences listing their sources
        const { account, residence } = options.residence;
        const missing = [
            ...(account.fields["kind"] === undefined ? [`${account.name}.kind`] : []),
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
    /** Read the source type a projection refers to. */
    sourceOf(options: ProjectedDefinition): ObjectType {
        return typeof options.from === "function" ? options.from() : options.from;
    },
};
