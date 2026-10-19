import { type JsonValue, schema } from "@destack/schema";
import type * as declaration from "../declare/database.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { Model } from "../query/model.ts";
import type { Table } from "../table/table.ts";
import type { TestDatabase } from "./database.ts";

/** A template's image in a run, which tests copy: its database and its value. */
export const TemplateImage = schema.object({
    /** Where the rows are. */
    source: schema.discriminatedUnion("dialect", [
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
    ]),
    /** What writing the rows returned, as JSON. */
    value: schema.json(),
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

/** Rows tests start with: a named write into a migrated database, once per run. */
export class TestTemplate<
    Value extends JsonValue = JsonValue,
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> {
    /** The template's name in its run, such as `stack.space`. */
    readonly name: string;
    /** The tables it migrates. */
    readonly tables: declaration.Database<Models> | readonly Table[];
    /** The dialects tests copy it in, every test dialect when absent. */
    readonly dialects: readonly Dialect[] | undefined;
    /** The value every copy shares, as its JSON form crosses from the run to the tests. */
    readonly value: schema.Schema<Value>;
    /** Write the rows into the migrated database, returning the value every copy shares. */
    readonly write: (storage: TestDatabase<Models>) => Promise<Value>;

    /** Define a template by its name, tables, value and write. */
    constructor(fields: {
        readonly name: string;
        readonly tables: declaration.Database<Models> | readonly Table[];
        readonly dialects?: readonly Dialect[];
        readonly value: schema.Schema<Value>;
        readonly write: (storage: TestDatabase<Models>) => Promise<Value>;
    }) {
        // keep the fields
        this.name = fields.name;
        this.tables = fields.tables;
        this.dialects = fields.dialects;
        this.value = fields.value;
        this.write = fields.write;
    }
}
