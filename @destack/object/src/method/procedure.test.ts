import { expect, test } from "@destack/test";
import { principal } from "@destack/access";
import { defineObject } from "../object/object.ts";
import { ListPage, scopeRoute } from "./procedure.ts";

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

test("route a type's scope by its scope type or a listed policy, refusing one unlisted policy", () => {
    // scope a type by account's space type through its policy, listed and alone
    const listed = defineObject({ name: "listed", plural: "listeds", scope: [principal.space] });
    const single = defineObject({ name: "single", plural: "singles", scope: principal.space });

    // route the listed one through a scope field, and refuse the single one
    expect([scopeRoute(listed), refusalOf(() => scopeRoute(single))]).toEqual([
        { prefix: "", field: "scope" },
        "object single: list a policy scope, or scope it by its type",
    ]);
});

/** Read the message a definition's refusal throws. */
function refusalOf(work: () => unknown): string {
    try {
        work();

        return "none";
    } catch (error) {
        return error instanceof Error ? error.message : String(error);
    }
}
