import { schema } from "@destack/schema";

/** The SQL dialect of a database. */
export const Dialect = schema.enum(["sqlite", "postgresql"]);

/** The SQL dialect of a database. */
export type Dialect = schema.Infer<typeof Dialect>;
