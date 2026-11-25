import { expect, test } from "@destack/test";
import { Search } from "./search.ts";

/** Mark the matched places of texts by brackets, with the score before them. */
function mark(query: string, ...texts: readonly string[]): string | undefined {
    const search = Search.of(query);
    const score = search.score(texts);
    const marked = texts.map((text) =>
        search
            .ranges(text)
            .reduceRight(
                (left, [start, end]) =>
                    `${left.slice(0, start)}[${left.slice(start, end)}]${left.slice(end)}`,
                text,
            ),
    );

    return score === undefined ? undefined : `${String(score)} ${marked.join(" | ")}`;
}

test("score the starts of words in any order, then text inside them, and mark where", () => {
    expect([
        mark("change status", "Change status"),
        mark("cha", "Change status"),
        mark("change sta", "Change status"),
        mark("status change", "Change status"),
        mark("sta cha", "Change status"),
        mark("priority", "Change status", "Priority"),
        mark("pri", "Change status", "High priority"),
        mark("sta pri", "Change status", "Priority"),
        mark("ange", "Change status"),
        mark("tatus", "Change status", "Estatus"),
        mark("zzz", "Change status"),
        mark("sta zzz", "Change status"),
        mark("", "Change status"),
    ]).toEqual([
        "10 [Change] [status]",
        "8 [Cha]nge status",
        "8 [Change] [sta]tus",
        "6 [Change] [status]",
        "4 [Cha]nge [sta]tus",
        "9 Change status | [Priority]",
        "3 Change status | High [pri]ority",
        "2 Change [sta]tus | [Pri]ority",
        "1 Ch[ange] status",
        "1 Change s[tatus] | Es[tatus]",
        undefined,
        undefined,
        "8 Change status",
    ]);
});

test("mark text in every script and where folding changes its length", () => {
    expect([
        mark("CAFÉ", "Le café"),
        mark("cafe", "Cafe\u0301s"),
        mark("fe", "Café"),
        mark("engine note", "engine-note"),
        mark("i̇st", "İstanbul"),
        mark("strasse", "Straße 12"),
        mark("stras", "Straße 12"),
        mark("a", "a b a"),
        mark("lodz", "Łódź"),
        mark("йод", "Йод"),
        mark("οδος", "Η οδός"),
        mark("한", "한국어"),
        mark("東京", "東京都庁"),
        mark("都庁", "東京都庁"),
        mark("庁", "東京都庁"),
        mark("東京 tower", "東京 Tower"),
        mark("买", "iPhone15买"),
        mark("ครับ", "สวัสดีครับ"),
        mark("हिन्दी", "मेरी हिन्दी"),
        mark("محمد", "مُحَمَّد"),
        mark("abc", "ＡＢＣ"),
        mark("finance", "ﬁnance"),
        mark("١", "١٢"),
    ]).toEqual([
        "6 Le [café]",
        "8 [Cafe\u0301]s",
        "1 Ca[fé]",
        "10 [engine]-[note]",
        "8 [İst]anbul",
        "8 [Straße] 12",
        "8 [Straß]e 12",
        "8 [a] b [a]",
        "10 [Łódź]",
        "10 [Йод]",
        "6 Η [οδός]",
        "8 [한]국어",
        "8 [東京]都庁",
        "6 東京[都庁]",
        "6 東京都[庁]",
        "10 [東京] [Tower]",
        "6 iPhone15[买]",
        "6 สวัสดี[ครับ]",
        "6 मेरी [हिन्दी]",
        "10 [مُحَمَّد]",
        "10 [ＡＢＣ]",
        "10 [ﬁnance]",
        "8 [١]٢",
    ]);
});

test("rank items, the best first and equal ones in their order", () => {
    const items = ["Order book", "Reorder", "Margins", "Order", "The orders", "The order form"];

    expect({
        order: Search.of("order").rank(items, (item) => [item]),
        blank: Search.of(" - ").rank(items, (item) => [item]),
        none: Search.of("zzz").rank(items, (item) => [item]),
    }).toEqual({
        order: ["Order", "Order book", "The order form", "The orders", "Reorder"],
        blank: items,
        none: [],
    });
});

test("cut the passage around the first match, whole words where there are spaces", () => {
    const text =
        "Two minutes of film.\nWe check the margins before the order form goes to the printer.\ufffcDone";

    expect([
        Search.of("margins order").excerpt(text, 40),
        Search.of("printer").excerpt(text, 40),
        Search.of("two").excerpt(text, 12),
        Search.of("done").excerpt(text),
        Search.of("手紙").excerpt(`東京都庁で手紙を買う。${"東京".repeat(20)}`, 12),
        Search.of("zzz").excerpt(text),
        Search.of("").excerpt(text),
    ]).toEqual([
        "We check the margins before the order…",
        "…goes to the printer.",
        "Two minutes…",
        "Done",
        "…京都庁で手紙を買う。東京…",
        undefined,
        undefined,
    ]);
});

test("choose the word someone most likely meant", () => {
    const terms = ["order", "ordering", "older", "orchid", "harbour", "harbours"].map(
        (term, index) => ({
            term,
            rows: index === 1 ? 9 : 1,
        }),
    );
    const closest = (word: string, isLast = false) => Search.closest(word, terms, isLast);

    expect({
        has: closest("order"),
        start: closest("orch"),
        left: closest("ordr"),
        swapped: closest("odrer"),
        plural: closest("orders"),
        two: closest("harbuors"),
        last: closest("ordri", true),
        short: closest("ord2"),
        far: closest("margin"),
    }).toEqual({
        has: "order",
        start: "orch",
        left: "order",
        swapped: "order",
        plural: "order",
        two: "harbours",
        last: "ordering",
        short: undefined,
        far: undefined,
    });
});

test("refuse a condition's search of no words or too many, and read the first typed ones", () => {
    const many = Array.from({ length: 33 }, (_each, index) => `w${String(index)}`).join(" ");

    expect(() => Search.parse(" - ")).toThrow(
        "search ' - ' is invalid: it has 0 words, not 1 to 32",
    );
    expect(() => Search.parse(many)).toThrow(
        `search '${many}' is invalid: it has 33 words, not 1 to 32`,
    );
    expect(Search.of(many).words.length).toBe(32);
    expect(Search.parse("Ada LOV").words).toEqual(["ada", "lov"]);
});
