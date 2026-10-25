import { type JsonValue, schema } from "@destack/schema";
import type * as declaration from "../declare/database.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { Model } from "../query/model.ts";
import type { Table } from "../table/table.ts";
import type { Simulation } from "@destack/environment/simulation";
import type { TestDatabase } from "./database.ts";

/** Where one database of a template's image keeps its rows. */
export const ImageSource = schema.discriminatedUnion("dialect", [
    schema.object({
        /** The dialect. */
        dialect: schema.literal("sqlite"),
        /** The file with the rows. */
        file: schema.string(),
    }),
    schema.object({
        /** The dialect. */
        dialect: schema.literal("postgresql"),
        /** The schema with the rows. */
        schema: schema.string(),
        /** The stored columns of each table, quoted and listed in declared order. */
        columns: schema.record(schema.string(), schema.string()),
    }),
]);
/** Where one database of a template's image keeps its rows. */
export type ImageSource = schema.Infer<typeof ImageSource>;

/** A template's image in a run, which tests copy: its databases by name and its value. */
export const TemplateImage = schema.object({
    /** Where each database's rows are, by the template's name for it. */
    sources: schema.record(schema.string(), ImageSource),
    /** What writing the rows returned, as JSON. */
    value: schema.json(),
    /** The template world's time once the write ended, in UTC epoch milliseconds. */
    end: schema.number(),
});
/** A template's image in a run, which tests copy. */
export type TemplateImage = schema.Infer<typeof TemplateImage>;

/** The images of a run's templates, by template name and dialect. */
export const ImageIndex = schema.record(
    schema.string(),
    schema.partialRecord(schema.enum(["sqlite", "postgresql"]), TemplateImage),
);
/** The images of a run's templates, by template name and dialect. */
export type ImageIndex = schema.Infer<typeof ImageIndex>;

/** The tables of one database a template writes. */
export type TemplateTables = declaration.Database | readonly Table[];

/** The databases a template writes, by name. */
export type TemplateDatabases = Readonly<Record<string, TemplateTables>>;

/** The models of one database's tables. */
export type ModelsOf<Tables> =
    Tables extends declaration.Database<infer Models> ? Models : Readonly<Record<string, Model>>;

/** The test databases of a template, by its name for each. */
export type TemplateStorage<Databases extends TemplateDatabases> = {
    readonly [Name in keyof Databases]: TestDatabase<ModelsOf<Databases[Name]>>;
};

/** The seed and start of the world a template's write runs in. */
export interface TemplateWorld {
    /** The seed of its randomness. */
    readonly seed: number;
    /** The time it starts at, in UTC epoch milliseconds. */
    readonly start: number;
}

/** Rows tests start with: a named write into migrated databases, once per run. */
export class TestTemplate<
    Value extends JsonValue = JsonValue,
    Databases extends TemplateDatabases = TemplateDatabases,
> {
    /** The template's name in its run, such as `stack.space`. */
    readonly name: string;
    /** The tables each of its databases migrates, by name. */
    readonly databases: Databases;
    /** The dialects tests copy it in, every test dialect when absent. */
    readonly dialects: readonly Dialect[] | undefined;
    /** The value every copy shares, as its JSON form crosses from the run to the tests. */
    readonly value: schema.Schema<Value>;
    /** The seed and start of the world the write runs in, which the run opens for it. */
    readonly world: TemplateWorld;
    /** Write the rows and files in the run's world, answering what every copy shares. */
    readonly write: (
        databases: TemplateStorage<Databases>,
        directory: string,
        simulation: Simulation,
    ) => Promise<Value>;

    /** Define a template by its name, databases, value and write. */
    constructor(fields: {
        readonly name: string;
        readonly databases: Databases;
        readonly dialects?: readonly Dialect[];
        readonly value: schema.Schema<Value>;
        readonly world: TemplateWorld;
        readonly write: (
            databases: TemplateStorage<Databases>,
            directory: string,
            simulation: Simulation,
        ) => Promise<Value>;
    }) {
        // store the fields
        this.name = fields.name;
        this.databases = fields.databases;
        this.dialects = fields.dialects;
        this.value = fields.value;
        this.world = fields.world;
        this.write = fields.write;
    }
}
