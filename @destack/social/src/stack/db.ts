import type { Table } from "@destack/db";
import { comment } from "../object/comment.ts";
import { favourite } from "../object/favourite.ts";
import { reaction } from "../object/reaction.ts";
import { receipt } from "../object/receipt.ts";

/** The tables of the comments, reactions, receipts and favourites on a space's objects, which the space's database keeps once. */
export const socialTables: readonly Table[] = [comment, reaction, receipt, favourite].flatMap(
    (object) => object.tables,
);
