import type { Table } from "@destack/db";
import { bucket } from "../object/bucket.ts";

/** The bucket facets of a space's resources. */
export const bucketTables: readonly Table[] = bucket.tables;
