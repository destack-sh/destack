import { expect, test } from "@destack/test";
import { ListPage } from "./procedure.ts";

test("read every row of a list across its pages, passing each page's cursor once", async () => {
    // answer three pages of a list, recording the cursors asked
    const pages: Readonly<Record<string, ListPage<number>>> = {
        first: { items: [1, 2], cursor: "second" },
        second: { items: [3], cursor: "third" },
        third: { items: [4, 5], cursor: null },
    };
    const asked: (string | undefined)[] = [];
    const list = async (input: { readonly scope: string; readonly cursor?: string }) => {
        asked.push(input.cursor);

        return pages[input.cursor ?? "first"] ?? { items: [], cursor: null };
    };

    expect({
        rows: await Array.fromAsync(ListPage.all(list, { scope: "space-1" })),
        asked,
    }).toEqual({
        rows: [1, 2, 3, 4, 5],
        asked: [undefined, "second", "third"],
    });
});
