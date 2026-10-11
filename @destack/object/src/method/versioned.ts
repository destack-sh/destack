import { schema } from "@destack/schema";
import type { ObjectType } from "../object/object.ts";
import type { Call } from "./call.ts";

/** Read the number following a parent's latest version. */
export async function nextVersion(
    object: ObjectType,
    parentId: string,
    call: Call,
): Promise<number> {
    // read the parent's highest number and count on from it
    const [latest] = await call
        .read(object.table)
        .rows({ parentId }, { orderBy: { number: "desc" }, limit: 1 });

    return latest === undefined ? 1 : schema.number().parse(latest["number"]) + 1;
}
