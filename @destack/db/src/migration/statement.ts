import type {
    ColumnDescription,
    ConstraintDescription,
    IndexDescription,
    TableDescription,
} from "../table/description.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { assertNever } from "../error/error.ts";
import { quote } from "../dialect/quote.ts";

/** Create a table; PostgreSQL adds foreign keys separately. */
export function createTable(table: TableDescription, name = table.name): string {
    // write the columns and constraints, with inline foreign keys only in SQLite
    const definitions = [
        ...table.columns.map((column) => columnDefinition(column, table.dialect)),
        ...table.constraints
            .filter((constraint) => constraint.kind !== "foreignKey" || table.dialect === "sqlite")
            .map(constraintDefinition),
    ];

    return `CREATE TABLE ${quote(name)} (\n    ${definitions.join(",\n    ")}\n)`;
}

/** Remove a PostgreSQL constraint. */
export function dropConstraint(table: string, name: string): string {
    return `ALTER TABLE ${quote(table)} DROP CONSTRAINT ${quote(name)}`;
}

/** Add a PostgreSQL constraint to an existing table. */
export function addConstraint(table: string, constraint: ConstraintDescription): string {
    return `ALTER TABLE ${quote(table)} ADD ${constraintDefinition(constraint)}`;
}

/** Drop a table. */
export function dropTable(name: string): string {
    return `DROP TABLE ${quote(name)}`;
}

/** Rename a table. */
export function renameTable(from: string, to: string): string {
    return `ALTER TABLE ${quote(from)} RENAME TO ${quote(to)}`;
}

/** Add a column to an existing table. */
export function addColumn(table: string, column: ColumnDescription, dialect: Dialect): string {
    return `ALTER TABLE ${quote(table)} ADD COLUMN ${columnDefinition(column, dialect)}`;
}

/** Drop a column. */
export function dropColumn(table: string, column: string): string {
    return `ALTER TABLE ${quote(table)} DROP COLUMN ${quote(column)}`;
}

/** Rename a column. */
export function renameColumn(table: string, from: string, to: string): string {
    return `ALTER TABLE ${quote(table)} RENAME COLUMN ${quote(from)} TO ${quote(to)}`;
}

/** Change a PostgreSQL column in place. */
export function alterColumn(
    table: string,
    from: ColumnDescription,
    to: ColumnDescription,
): string[] {
    // name the table and column
    const target = quote(table);
    const column = quote(to.name);
    const statements: string[] = [];

    // convert the values to another type
    if (from.type !== to.type) {
        statements.push(
            `ALTER TABLE ${target} ALTER COLUMN ${column} TYPE ${typed(to)} USING ${column}::${to.type}`,
        );
    }
    // compare the values under another collation
    else if (from.collation !== to.collation) {
        statements.push(`ALTER TABLE ${target} ALTER COLUMN ${column} TYPE ${typed(to)}`);
    }

    // alter the nullability and default
    if (from.nullable !== to.nullable) {
        statements.push(
            `ALTER TABLE ${target} ALTER COLUMN ${column} ${to.nullable ? "DROP" : "SET"} NOT NULL`,
        );
    }
    if (from.default !== to.default) {
        statements.push(
            to.default === undefined
                ? `ALTER TABLE ${target} ALTER COLUMN ${column} DROP DEFAULT`
                : `ALTER TABLE ${target} ALTER COLUMN ${column} SET DEFAULT ${to.default}`,
        );
    }

    return statements;
}

/** Create an index. */
export function createIndex(table: string, index: IndexDescription): string {
    // write the columns and predicate
    const columns = index.columns
        .map((entry) => ("column" in entry ? quote(entry.column) : `(${entry.expression})`))
        .join(", ");
    const nulls = index.nullsNotDistinct === true ? " NULLS NOT DISTINCT" : "";
    const where = index.where === undefined ? "" : ` WHERE ${index.where}`;

    return `CREATE ${index.unique ? "UNIQUE " : ""}INDEX ${quote(index.name)} ON ${quote(table)} (${columns})${nulls}${where}`;
}

/** Drop an index. */
export function dropIndex(name: string): string {
    return `DROP INDEX ${quote(name)}`;
}

/** Name the table a SQLite rebuild fills before it takes a table's place. */
function staging(name: string): string {
    return `${name}__rebuild`;
}

/** A SQLite table to rebuild as described, with the columns its rows keep. */
export interface TableRebuild {
    /** The table as it is after the rebuild. */
    readonly table: TableDescription;
    /** The columns copied from the table's rows. */
    readonly copied: readonly string[];
    /** The new columns filled from each row by a conversion's SQL assignment, by column. */
    readonly filled: Readonly<Record<string, string>>;
}

/** Rebuild SQLite tables with every table referencing them, under enforced foreign keys. */
export function rebuildTables(rebuilds: readonly TableRebuild[]): string[] {
    // stage every table first, each referencing the staged tables it rebuilds with
    const rebuilt = new Set(rebuilds.map(({ table }) => table.name));
    const staged = rebuilds.map(({ table }) => {
        const constraints = table.constraints.map((constraint) =>
            constraint.kind === "foreignKey" && rebuilt.has(constraint.table)
                ? { ...constraint, table: staging(constraint.table) }
                : constraint,
        );

        return createTable({ ...table, constraints }, staging(table.name));
    });

    // copy each table's kept columns once every staged table it references exists
    const copies = rebuilds.map(({ table, copied, filled }) => {
        // copy the kept columns and fill the converted new ones from each row
        const columns = [...copied, ...Object.keys(filled)].map(quote).join(", ");
        const values = [...copied.map(quote), ...Object.values(filled)].join(", ");
        const target = quote(staging(table.name));

        return `INSERT INTO ${target} (${columns}) SELECT ${values} FROM ${quote(table.name)}`;
    });

    // drop each table after the tables referencing it, so no drop deletes another's rows
    const pending = new Map(rebuilds.map(({ table }) => [table.name, table]));
    const dropped: string[] = [];
    while (pending.size > 0) {
        const referenced = new Set(
            [...pending.values()].flatMap((table) =>
                table.constraints.flatMap((constraint) =>
                    constraint.kind === "foreignKey" && constraint.table !== table.name
                        ? [constraint.table]
                        : [],
                ),
            ),
        );
        const free = [...pending.keys()].filter((name) => !referenced.has(name));
        const next = free.length > 0 ? free : [...pending.keys()].slice(0, 1);
        for (const name of next) {
            dropped.push(dropTable(name));
            pending.delete(name);
        }
    }

    // rename each staged table, which points the references at it back to its name
    return [
        ...staged,
        ...copies,
        ...dropped,
        ...rebuilds.map(({ table }) => renameTable(staging(table.name), table.name)),
        ...rebuilds.flatMap(({ table }) =>
            table.indexes.map((index) => createIndex(table.name, index)),
        ),
    ];
}

/** Write one column's definition. */
function columnDefinition(column: ColumnDescription, dialect: Dialect): string {
    // write the type, default and nullability
    const terms = [quote(column.name), typed(column)];
    if (column.generated !== undefined) {
        terms.push(
            `GENERATED ALWAYS AS (${column.generated.expression}) ${storage(column, dialect)}`,
        );
    } else if (column.default !== undefined) {
        terms.push(`DEFAULT ${column.default}`);
    }
    if (!column.nullable) {
        terms.push("NOT NULL");
    }

    return terms.join(" ");
}

/** Write a column's type with its collation. */
function typed(column: ColumnDescription): string {
    return column.collation === undefined
        ? column.type
        : `${column.type} COLLATE ${quote(column.collation)}`;
}

/** Write a generated column's storage. */
function storage(column: ColumnDescription, dialect: Dialect): string {
    // store every PostgreSQL generated column
    if (dialect === "postgresql") {
        return "STORED";
    }
    // add the declared SQLite storage
    else if (dialect === "sqlite") {
        return column.generated?.mode === "virtual" ? "VIRTUAL" : "STORED";
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }
}

/** Write a constraint's definition. */
function constraintDefinition(constraint: ConstraintDescription): string {
    const name = `CONSTRAINT ${quote(constraint.name)}`;

    // write a check
    if (constraint.kind === "check") {
        return `${name} CHECK (${constraint.expression})`;
    }
    // write a foreign key with its actions
    else if (constraint.kind === "foreignKey") {
        const onDelete = constraint.onDelete?.toUpperCase();
        const onUpdate = constraint.onUpdate?.toUpperCase();
        const actions = [
            onDelete === undefined ? "" : ` ON DELETE ${onDelete}`,
            onUpdate === undefined ? "" : ` ON UPDATE ${onUpdate}`,
        ].join("");
        const references = `${quote(constraint.table)} (${names(constraint.references)})`;

        return `${name} FOREIGN KEY (${names(constraint.columns)}) REFERENCES ${references}${actions} DEFERRABLE INITIALLY IMMEDIATE`;
    }
    // write a primary key or unique constraint
    else {
        const kind = constraint.kind === "primaryKey" ? "PRIMARY KEY" : "UNIQUE";

        return `${name} ${kind} (${names(constraint.columns)})`;
    }
}

/** Write a quoted column list. */
function names(columns: readonly string[]): string {
    return columns.map(quote).join(", ");
}
