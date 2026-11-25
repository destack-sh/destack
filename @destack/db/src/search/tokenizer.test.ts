import { expect, test } from "@destack/test";
import { Tokenizer } from "./tokenizer.ts";

/** List a text's tokens: its words, those of scripts without spaces as pairs. */
function tokensOf(text: string): string[] {
    return Tokenizer.words(text).flatMap((word) => Tokenizer.pairs(word));
}

test("cut text into words, and scripts without spaces into pairs", () => {
    expect([
        tokensOf("The order-book, No. 12"),
        tokensOf("東京都庁 Tower 庁"),
        tokensOf("iPhone15を買う"),
        tokensOf(" - "),
    ]).toEqual([
        ["the", "order", "book", "no", "12"],
        ["東京", "京都", "都庁", "庁", "tower", "庁"],
        ["iphone15", "を買", "買う", "う"],
        [],
    ]);
});

test("leave out the diacritics and marks that are no letters, and list every digit", () => {
    // compare the marks and digits of Unicode below U+10000 with those listed
    const isLeftOut = new RegExp(`^[${Tokenizer.marks}]$`, "u");
    const isDigit = new RegExp(`^[0-9${Tokenizer.digits}]$`, "u");
    const wrong: string[] = [];
    for (let point = 0x20; point < 0xff00; point += 1) {
        const character = String.fromCodePoint(point);
        const isMark = /^\p{M}$/u.test(character) && point >= 0x300;
        const isNoLetter = /^(?:[\p{Diacritic}\u0300-\u036f]|\P{Alphabetic})$/u.test(character);
        if (
            (isMark && isLeftOut.test(character) !== isNoLetter) ||
            /^\p{Nd}$/u.test(character) !== isDigit.test(character)
        ) {
            wrong.push(point.toString(16));
        }
    }

    expect(wrong).toEqual([]);
});

test("fold a folded text to itself, and one character by character alike", () => {
    const texts = ["Straße ÆØÅ", "ΟΔΟΣ οδός", "हिन्दी", "สวัสดี", "한국어", "ＡＢＣ ﬁ", "İstanbul ı"];
    const folded = texts.map((text) => Tokenizer.fold(text));

    expect(folded.map((text) => Tokenizer.fold(text))).toEqual(folded);
    expect(texts.map((text) => Tokenizer.located(text).text)).toEqual(folded);
});
