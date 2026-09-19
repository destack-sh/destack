import { defineDatabase } from "@destack/db/declare";

/** The space's shared application database. */
export const database = defineDatabase({ name: "main", spec: { dialect: "sqlite" } });
