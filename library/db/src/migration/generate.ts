import { generate, type GenerateOptions } from "drizzle-kit/cli";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Dialect } from "../dialect/dialect.ts";
import type { DatabaseSchema } from "../schema/schema.ts";

/** Schema compiler modules indexed by SQL dialect. */
const SCHEMA_COMPILERS = {
    sqlite: { module: "../turso/compiler.ts", export: "SQLiteSchemaCompiler" },
    postgresql: { module: "../postgres/compiler.ts", export: "PostgresSchemaCompiler" },
} satisfies Record<Dialect, { module: string; export: string }>;

/** Source schema and dialect used to generate committed Drizzle migrations. */
export interface MigrationGeneration {
    /** The module exporting the database schema. */
    readonly module: URL;
    /** The exported DatabaseSchema name. */
    readonly export: string;
    /** The SQL dialect whose history is updated. */
    readonly dialect: Dialect;
    /** The migration name supplied to Drizzle Kit. */
    readonly name?: string;
    /** Explicit rename decisions supplied to Drizzle Kit. */
    readonly hints?: GenerateOptions["hints"];
}

/** Generate SQL and snapshots from a source schema using Drizzle Kit. */
export async function generateMigrations(request: MigrationGeneration) {
    // load the declaration to determine its tables and migration directory
    const module = await import(request.module.href);
    const definition = module[request.export] as DatabaseSchema | undefined;
    if (!definition) throw new TypeError(`Missing database schema export: ${request.export}.`);
    const names = Object.keys(definition.tables).map((_, index) => `table${index}`);
    const directory = await mkdtemp(join(tmpdir(), "destack-migration-"));
    let result: Awaited<ReturnType<typeof generate>>;

    // expose native tables through the module format accepted by Drizzle Kit
    try {
        const path = join(directory, "schema.mjs");
        const compiler = SCHEMA_COMPILERS[request.dialect];
        const implementation = new URL(compiler.module, import.meta.url);
        const declaration = new URL("../schema/schema.ts", import.meta.url);
        const source = [
            `import * as module from ${JSON.stringify(request.module.href)};`,
            `import { ${compiler.export} } from ${JSON.stringify(implementation.href)};`,
            `import { orderSchemas } from ${JSON.stringify(declaration.href)};`,
            `const definition = module[${JSON.stringify(request.export)}];`,
            `const schema = new ${compiler.export}(orderSchemas([definition]).flatMap(schema => Object.values(schema.tables)));`,
            `const tables = Object.values(definition.tables).map(table => schema.table(table));`,
            `export const [${names.join(", ")}] = tables;`,
        ].join("\n");
        await writeFile(path, source, "utf8");

        // let Drizzle retain snapshots, generate SQL, and report unresolved renames
        result = await generate({
            dialect: request.dialect,
            schema: path,
            out: join(fileURLToPath(definition.migrations), request.dialect),
            name: request.name,
            hints: request.hints,
        });
    } catch (error) {
        // retain the generation failure if temporary-file cleanup also fails
        try {
            await rm(directory, { recursive: true });
        } catch (cleanup) {
            throw new AggregateError([error, cleanup], "Migration generation and cleanup failed.");
        }
        throw error;
    }

    await rm(directory, { recursive: true });

    return result;
}
