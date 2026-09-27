import type { Dialect } from "./dialect.ts";

/** Quote a declared SQL identifier without accepting executable SQL. */
export function quote(name: string): string {
    return `"${name.replaceAll('"', '""')}"`;
}

/** Quote a string as an SQL literal. */
export function literal(value: string): string {
    return `'${value.replaceAll("'", "''")}'`;
}

/** Compare a column with a value in a dialect, null matching null. */
export function condition(value: string | number | boolean | null, dialect: Dialect): string {
    // match null by identity
    if (value === null) {
        return "IS NULL";
    }
    // quote text
    else if (typeof value === "string") {
        return `= ${literal(value)}`;
    }
    // write booleans as each dialect stores them
    else if (typeof value === "boolean") {
        return dialect === "sqlite" ? `= ${value ? 1 : 0}` : `= ${value}`;
    }
    // write numbers as they are
    else {
        return `= ${value}`;
    }
}
