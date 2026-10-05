import { schema } from "@destack/schema";
import { type DatabaseConnection, desc, eq, TABLE } from "@destack/db";
import type { ObjectType } from "../object/object.ts";

/** Read the number following a parent's latest version. */
export async function nextVersion(
    object: ObjectType,
    parentId: string,
    database: DatabaseConnection,
): Promise<number> {
    // read the parent's highest number and count on from it
    const table = object.table;
    const [latest] = await database
        .select({ number: table[TABLE].column("number") })
        .from(table)
        .where(eq(table[TABLE].column("parentId"), parentId))
        .orderBy(desc(table[TABLE].column("number")))
        .limit(1);

    return latest === undefined ? 1 : schema.number().parse(latest.number) + 1;
}
