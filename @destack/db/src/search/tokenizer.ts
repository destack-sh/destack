/** Code points as hexadecimal ranges with spaces between, as Unicode's data files write them. */
type Ranges = string;

/** The marks folding leaves out below U+10000: diacritics, and others that are no letters. */
const MARKS: Ranges = `
    ad 200c-200d 2060 feff
    300-36f 483-489 591-5bd 5bf 5c1-5c2 5c4-5c5 5c7 64b-652 657-658 6df-6e0 6ea-6ec
    730-74a 7a6-7b0 7eb-7f3 7fd 818-819 82d 859-85b 898-89f 8ca-8d3 8e0-8e1 8e3-8fe 93c
    94d 951-954 9bc 9cd 9fe a3c a4d abc acd afd-aff b3c b4d-b55 bcd c3c c4d cbc ccd
    d3b-d3c d4d dca e3a e47-e4c e4e eba ec8-ecc ece f18-f19 f35 f37 f39 f3e-f3f f82-f84
    f86-f87 fc6 1037 1039-103a 1063-1064 1069-106d 1087-108d 108f 109a-109b 135d-135f
    1714-1715 1734 17b4-17b5 17c9-17d3 17dd 180b-180d 180f 1939-193b 1a60 1a75-1a7f
    1ab0-1abe 1ac1-1acb 1acf-1aeb 1b34 1b44 1b6b-1b73 1baa-1bab 1be6 1bf2-1bf3 1c36-1c37
    1cd0-1cd2 1cd4-1ce8 1ced 1cf4 1cf7-1cf9 1dc0-1dd2 1df5-1dff 20d0-20f0 2cef-2cf1 2d7f
    302a-302f 3099-309a a66f-a672 a67c-a67d a6f0-a6f1 a806 a82c a8c4 a8e0-a8f1 a92b-a92d
    a953 a9b3 a9c0 a9e5 aa7b-aa7d aabf aac1 aaf6 abec-abed fb1e fe00-fe0f fe20-fe2f`;

/** The scripts written without spaces: Thai, Lao, Myanmar, Khmer, the kana and Han. */
const UNSPACED: Ranges = `
    e00-eff 1000-109f 1780-17ff 3005-3007 3041-30ff 31f0-31ff 3400-4dbf 4e00-9fff
    20000-3ffff`;

/** The decimal digits below U+10000 beside ASCII's, which PostgreSQL takes for no digits. */
const DIGITS: Ranges = `
    660-669 6f0-6f9 7c0-7c9 966-96f 9e6-9ef a66-a6f ae6-aef b66-b6f be6-bef c66-c6f ce6-cef
    d66-d6f de6-def e50-e59 ed0-ed9 f20-f29 1040-1049 1090-1099 17e0-17e9 1810-1819 1946-194f
    19d0-19d9 1a80-1a89 1a90-1a99 1b50-1b59 1bb0-1bb9 1c40-1c49 1c50-1c59 a620-a629 a8d0-a8d9
    a900-a909 a9d0-a9d9 a9f0-a9f9 aa50-aa59 abf0-abf9`;

/** The letters with a stroke, a hook or no dot, and those folding writes, as ICU folds them. */
const LETTERS = { from: "øłđðħŧıς", to: "olddhtiσ" };

/** The letters that are two, as Unicode's case folding and ICU's folding write them. */
const LIGATURES: readonly (readonly [letter: string, pair: string])[] = [
    ["ß", "ss"],
    ["æ", "ae"],
    ["œ", "oe"],
    ["þ", "th"],
];

/** A text of ASCII alone. */
const ASCII = /^[\0-\x7f]*$/u;

/** A folded text with where its characters come from. */
export interface FoldedText {
    /** The folded text. */
    readonly text: string;
    /** The place of each of its characters in the text, and the one after the last. */
    readonly places: readonly number[];
}

/** How search reads text, alike in SQLite, PostgreSQL and memory. */
export const Tokenizer = {
    /** The characters folding leaves out, as a class of regular expressions. */
    marks: classOf(MARKS),

    /** The letters and digits of words, as a class of regular expressions. */
    letter: `\\p{Alphabetic}0-9${classOf(DIGITS)}`,

    /** The characters of scripts without spaces, as a class of regular expressions. */
    unspaced: classOf(UNSPACED),

    /** The digits beside ASCII's, as a class of regular expressions. */
    digits: classOf(DIGITS),

    /** The letters folding writes as another, and those others in the same order. */
    letters: LETTERS,

    /** The letters folding writes as two. */
    ligatures: LIGATURES,

    /** Fold a text: its case, width, accents, vowel points, and letter variants. */
    fold(text: string): string {
        // lower ASCII, which has none of the others
        if (ASCII.test(text)) {
            return text.toLowerCase();
        }

        return text
            .normalize("NFKD")
            .toLowerCase()
            .replace(MARK, "")
            .replace(VARIANT, (letter) => VARIANTS.get(letter) ?? letter);
    },

    /** List a text's folded words, each of scripts without spaces or of others. */
    words(text: string): string[] {
        return (Tokenizer.fold(text).match(WORD) ?? []).flatMap((word) => word.match(RUN) ?? []);
    },

    /** Write a text's words, each with a space around. */
    line(text: string): string {
        return ` ${Tokenizer.words(text).join(" ")} `;
    },

    /** Cut a word of a script without spaces into pairs and its last character, leave another. */
    pairs(word: string): string[] {
        if (!Tokenizer.isUnspaced(word)) {
            return [word];
        }
        const characters = Array.from(word);

        return characters.map((character, index) => character + (characters[index + 1] ?? ""));
    },

    /** Report whether a word is of a script without spaces. */
    isUnspaced(word: string): boolean {
        return UNSPACED_START.test(word);
    },

    /** Fold a text with the place of each folded character, and the one after the last. */
    located(text: string): FoldedText {
        // fold character by character, which a whole text folds to but for its marks' order
        const places: number[] = [];
        let folded = "";
        let place = 0;
        for (const character of text) {
            const piece = Tokenizer.fold(character);
            folded += piece;
            for (let unit = 0; unit < piece.length; unit += 1) {
                places.push(place);
            }
            place += character.length;
        }

        return { text: folded, places: [...places, text.length] };
    },
};

/** The marks folding leaves out. */
const MARK = new RegExp(`[${Tokenizer.marks}]`, "gu");

/** The letters folding writes as others. */
const VARIANTS: ReadonlyMap<string, string> = new Map([
    ...Array.from(Tokenizer.letters.from, (letter, index): [string, string] => [
        letter,
        Array.from(Tokenizer.letters.to)[index] ?? letter,
    ]),
    ...LIGATURES,
]);

/** The characters folding writes as others. */
const VARIANT = new RegExp(`[${[...VARIANTS.keys()].join("")}]`, "gu");

/** A run of letters and digits. */
const WORD = new RegExp(`[${Tokenizer.letter}]+`, "gu");

/** A run of a script without spaces, or of others. */
const RUN = new RegExp(`[${Tokenizer.unspaced}]+|[^${Tokenizer.unspaced}]+`, "gu");

/** A character of a script without spaces at the start. */
const UNSPACED_START = new RegExp(`^[${Tokenizer.unspaced}]`, "u");

/** Write ranges as a class of regular expressions, in the characters themselves. */
function classOf(ranges: Ranges): string {
    return ranges
        .trim()
        .split(/\s+/u)
        .map((range) =>
            range
                .split("-")
                .map((point) => String.fromCodePoint(Number.parseInt(point, 16)))
                .join("-"),
        )
        .join("");
}
