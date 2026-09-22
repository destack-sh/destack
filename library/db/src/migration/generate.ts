import { generate, type GenerateOptions } from "drizzle-kit/cli";
import { mkdtemp, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Dialect } from "../dialect/dialect.ts";
import type { DatabaseSchema } from "../schema/schema.ts";
import { TreeDescription } from "../inspect/tree.ts";
import { TreeMigration } from "../tree/migration.ts";
import { DatabaseError } from "../error/error.ts";

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
    if (!definition) {
        throw new DatabaseError(
            "INVALID_MIGRATION",
            `missing database schema export: ${request.export}`,
        );
    }
    const output = join(fileURLToPath(definition.migrations), request.dialect);
    const previous = await readTrees(output);
    const trees = new TreeMigration(definition.trees ?? [], previous, request.dialect);
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
            out: output,
            name: request.name,
            hints: request.hints,
        });

        // create a custom migration when only tree maintenance changed
        if (result.status === "no_changes" && trees.changed) {
            result = await generate({
                dialect: request.dialect,
                schema: path,
                out: output,
                name: request.name,
                custom: true,
            });
        }

        // commit tree maintenance with the ordinary SQL and schema snapshot
        if (result.status === "ok" && "migration_path" in result) {
            if (trees.before.length > 0 || trees.after.length > 0) {
                const source = await readFile(result.migration_path, "utf8");
                const statements = [...trees.before, source, ...trees.after];
                await writeFile(
                    result.migration_path,
                    `${statements.join("\n--> statement-breakpoint\n")}\n`,
                );
            }
            if (trees.trees.length > 0 || previous.length > 0) {
                await writeFile(
                    join(dirname(result.migration_path), "trees.json"),
                    `${JSON.stringify(trees.trees, null, 4)}\n`,
                );
            }

            // retain the historical tree descriptions in the committed data migration
            if (trees.rebuild.length > 0) {
                const source = [
                    'import type { DatabaseConnection } from "@destack/db";',
                    'import { rebuildTree } from "@destack/db/tree";',
                    "",
                    "/** Populate the ancestor indexes declared by this migration. */",
                    "export async function migrate(database: DatabaseConnection): Promise<void> {",
                    ...trees.rebuild.map(
                        (tree) => `    await rebuildTree(database, ${JSON.stringify(tree)});`,
                    ),
                    "}",
                    "",
                ].join("\n");
                await writeFile(join(dirname(result.migration_path), "migration.ts"), source);
            }
        }
    } catch (error) {
        // retain the generation failure if temporary-file cleanup also fails
        try {
            await rm(directory, { recursive: true });
        } catch (cleanup) {
            throw new AggregateError([error, cleanup], "migration generation and cleanup failed");
        }
        throw error;
    }

    await rm(directory, { recursive: true });

    return result;
}

/** Read the tree definitions committed with the latest migration. */
async function readTrees(directory: string): Promise<TreeDescription[]> {
    let entries;
    try {
        entries = await readdir(directory, { withFileTypes: true });
    } catch (error) {
        if (error instanceof Error && "code" in error && error.code === "ENOENT") {
            return [];
        }
        throw error;
    }

    // older schemas have no tree declarations until their first tree migration
    const names = entries
        .filter((entry) => entry.isDirectory() && /^\d+_/.test(entry.name))
        .map((entry) => entry.name)
        .sort();
    // custom SQL migrations may leave the previously recorded tree declarations unchanged
    for (const name of names.reverse()) {
        let source;
        try {
            source = await readFile(join(directory, name, "trees.json"), "utf8");
        } catch (error) {
            if (error instanceof Error && "code" in error && error.code === "ENOENT") {
                continue;
            }
            throw error;
        }

        return TreeDescription.array().parse(JSON.parse(source));
    }

    return [];
}
