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

/** What a build leaves of a template in a run: its rows' places, its files, value and end. */
export const ImageRecord = schema.object({
    /** Where each database's rows are, by the template's name for it. */
    sources: schema.record(schema.string(), ImageSource),
    /** Where each database the build opened as it ran keeps its rows, by its key. */
    volumes: schema.record(schema.string(), ImageSource),
    /** The directory with the files the build wrote. */
    files: schema.string(),
    /** What the build returned, as JSON. */
    value: schema.json(),
    /** The template world's time once the build ended, in UTC epoch milliseconds. */
    end: schema.number(),
});
/** What a build leaves of a template in a run. */
export type ImageRecord = schema.Infer<typeof ImageRecord>;

/** One database a template builds: its declaration, or its tables. */
export type TemplateDatabase = declaration.Database | readonly Table[];

/** The databases a template builds, by name. */
export type TemplateDatabases = Readonly<Record<string, TemplateDatabase>>;

/** The models of one database's tables. */
export type ModelsOf<Tables> =
    Tables extends declaration.Database<infer Models> ? Models : Readonly<Record<string, Model>>;

/** The test databases of a template, by its name for each. */
export type TemplateStorage<Databases extends TemplateDatabases> = {
    readonly [Name in keyof Databases]: TestDatabase<ModelsOf<Databases[Name]>>;
};

/** The seed and start of the world a template's build runs in. */
export interface TemplateWorld {
    /** The seed of its randomness. */
    readonly seed: number;
    /** The time it starts at, in UTC epoch milliseconds. */
    readonly start: number;
}

/** What a build runs in: databases, a directory and a world, its parent's when it has one. */
export interface TemplateBuild<Databases extends TemplateDatabases = TemplateDatabases> {
    /** The template's databases, a parent's with its rows. */
    readonly databases: TemplateStorage<Databases>;
    /** The directory for the files the build writes, a parent's with its files. */
    readonly directory: string;
    /** The world the build runs in, a parent's forked at its end. */
    readonly simulation: Simulation;
    /** Open a database the build keeps by a key, a parent's with its rows. */
    readonly open: (key: string) => Promise<TestDatabase>;
    /** What the parent's build returned, absent for a template built from nothing. */
    readonly parent: JsonValue | undefined;
}

/** A world tests start from: a named build on its parent's image, built when first forked. */
export class TestTemplate<
    Value extends JsonValue = JsonValue,
    Databases extends TemplateDatabases = TemplateDatabases,
> {
    /** The template's name in its run, such as `stack.space`. */
    readonly name: string;
    /** The template whose image the build starts from, absent for one built from nothing. */
    readonly from: TestTemplate<JsonValue, Databases> | undefined;
    /** The tables each of its databases migrates, by name, its parent's when it has one. */
    readonly databases: Databases;
    /** The dialects tests fork it in, every test dialect when absent. */
    readonly dialects: readonly Dialect[] | undefined;
    /** The value every fork shares, as its JSON form is kept with the image. */
    readonly value: schema.Schema<Value>;
    /** The tables of each database the build opens as it runs, none when absent. */
    readonly volume: TemplateDatabase | undefined;
    /** The seed and start of the world the first build of its line runs in. */
    readonly world: TemplateWorld;
    /** Build the rows and files in the world, answering what every fork shares. */
    readonly build: (world: TemplateBuild<Databases>) => Promise<Value>;

    /** Define a template by its name, its parent or its databases, its value and its build. */
    constructor(
        fields: {
            readonly name: string;
            readonly dialects?: readonly Dialect[];
            readonly value: schema.Schema<Value>;
            readonly build: (world: TemplateBuild<Databases>) => Promise<Value>;
        } & (
            | {
                  /** The template whose image the build starts from. */
                  readonly from: TestTemplate<JsonValue, Databases>;
              }
            | {
                  /** The tables each of its databases migrates, by name. */
                  readonly databases: Databases;
                  /** The tables of each database the build opens as it runs. */
                  readonly volume?: TemplateDatabase;
                  /** The seed and start of the world the build runs in. */
                  readonly world: TemplateWorld;
              }
        ),
    ) {
        // store the fields, a parent giving the databases, volume tables and world
        this.name = fields.name;
        this.dialects = fields.dialects;
        this.value = fields.value;
        this.build = fields.build;
        if ("from" in fields) {
            this.from = fields.from;
            this.databases = fields.from.databases;
            this.volume = fields.from.volume;
            this.world = fields.from.world;
        } else {
            this.from = undefined;
            this.databases = fields.databases;
            this.volume = fields.volume;
            this.world = fields.world;
        }
    }
}
