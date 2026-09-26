import type {
    ColumnDescription,
    ConstraintDescription,
    IndexDescription,
    TableDescription,
} from "../inspect/table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { assertNever } from "../error/error.ts";
import { quote } from "../dialect/quote.ts";

/** Create a table with its columns and constraints; PostgreSQL adds foreign keys separately. */
export function createTable(table: TableDescription, name = table.name): string {
    // write the columns and constraints, keeping foreign keys inline only in SQLite
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

/** Change a PostgreSQL column's type, nullability and default in place. */
export function alterColumn(
    table: string,
    from: ColumnDescription,
    to: ColumnDescription,
): string[] {
    // alter the type, nullability and default
    const target = quote(table);
    const column = quote(to.name);
    const statements: string[] = [];
    if (from.type !== to.type) {
        statements.push(
            `ALTER TABLE ${target} ALTER COLUMN ${column} TYPE ${to.type} USING ${column}::${to.type}`,
        );
    }
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
    // write the indexed columns and the partial predicate
    const columns = index.columns
        .map((entry) => ("column" in entry ? quote(entry.column) : `(${entry.expression})`))
        .join(", ");
    const where = index.where === undefined ? "" : ` WHERE ${index.where}`;

    return `CREATE ${index.unique ? "UNIQUE " : ""}INDEX ${quote(index.name)} ON ${quote(table)} (${columns})${where}`;
}

/** Drop an index. */
export function dropIndex(name: string): string {
    return `DROP INDEX ${quote(name)}`;
}

/** Rebuild a SQLite table into a new definition, copying the columns both share. */
export function rebuildTable(
    table: TableDescription,
    copied: ReadonlyMap<string, string>,
): string[] {
    // create the new definition beside the old table and copy shared columns
    const staging = `${table.name}__rebuild`;
    const targets = [...copied.keys()].map(quote).join(", ");
    const sources = [...copied.values()].map(quote).join(", ");

    return [
        createTable(table, staging),
        `INSERT INTO ${quote(staging)} (${targets}) SELECT ${sources} FROM ${quote(table.name)}`,
        dropTable(table.name),
        renameTable(staging, table.name),
        ...table.indexes.map((index) => createIndex(table.name, index)),
    ];
}

/** Write one column's definition. */
function columnDefinition(column: ColumnDescription, dialect: Dialect): string {
    // write the type, default and nullability
    const parts = [quote(column.name), column.type];
    if (column.generated) {
        parts.push(
            `GENERATED ALWAYS AS (${column.generated.expression}) ${storage(column, dialect)}`,
        );
    } else if (column.default !== undefined) {
        parts.push(`DEFAULT ${column.default}`);
    }
    if (!column.nullable) {
        parts.push("NOT NULL");
    }

    return parts.join(" ");
}

/** Write a generated column's storage, where PostgreSQL stores every generated column. */
function storage(column: ColumnDescription, dialect: Dialect): string {
    // store every PostgreSQL generated column
    if (dialect === "postgresql") {
        return "STORED";
    }
    // keep the declared SQLite storage
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
    // write a foreign key with the referential actions it declares, checked per statement unless a transaction defers it
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
