import { included, type Item, type Query } from "@destack/sync";
import { present } from "@destack/schema";
import type { ObjectType } from "../object/index.ts";
import { Chunk } from "../text/chunk.ts";
import { CHUNKS } from "../text/table.ts";

/** Rows with their chunks replaced by texts, each row and list presented once while unchanged. */
export class TextPresenter {
    /** The presented rows, by the read row they present. */
    readonly #rows: WeakMap<object, Item>;
    /** The presented lists, by the read list they present. */
    readonly #lists: WeakMap<object, readonly Item[]>;

    /** Present nothing yet. */
    constructor() {
        this.#rows = new WeakMap();
        this.#lists = new WeakMap();
    }

    /** Replace each row's chunks by its texts, reusing presented rows and lists if unchanged. */
    present(rows: readonly Item[], query: Query, objects: readonly ObjectType[]): readonly Item[] {
        // take a list presented before
        const known = this.#lists.get(rows);
        if (known !== undefined) {
            return known;
        }

        // present each row once
        const owner = objects.find((object) => object.table === query.table);
        const listed = rows.map((row) => {
            const presented = this.#rows.get(row) ?? this.#row(row, query, objects, owner);
            this.#rows.set(row, presented);

            return presented;
        });
        this.#lists.set(rows, listed);

        return listed;
    }

    /** Present one item's includes and texts. */
    #row(
        item: Item,
        query: Query,
        objects: readonly ObjectType[],
        owner: ObjectType | undefined,
    ): Item {
        // present each include's items, leaving out the chunks
        const presented = Object.fromEntries(
            Object.entries(item.with).flatMap(([name, items]) =>
                name === CHUNKS
                    ? []
                    : [[name, this.present(items, included(query, name).query, objects)]],
            ),
        );

        // assemble a text owner's texts from the chunks its query includes
        const row =
            owner === undefined || owner.text.length === 0
                ? item.row
                : {
                      ...item.row,
                      ...Chunk.text(
                          owner,
                          Chunk.included(
                              present(item.with[CHUNKS], `the chunks of ${owner.name}`).map(
                                  (entry) => entry.row,
                              ),
                          ),
                      ),
                  };

        return { row, with: presented, measures: item.measures };
    }
}
