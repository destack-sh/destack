/** Quote a declared SQL identifier without accepting executable SQL. */
export function quote(name: string): string {
    return `"${name.replaceAll('"', '""')}"`;
}

/** Quote a string as an SQL literal. */
export function literal(value: string): string {
    return `'${value.replaceAll("'", "''")}'`;
}
