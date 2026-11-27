import { canonicalize, type JsonValue, schema } from "@destack/schema";
import type { Subject } from "@destack/sync";
import type { TextSpan } from "../sequence/index.ts";
import { Mention } from "./mention.ts";

/** The value of a link. */
const Link = schema.looseObject({ href: schema.string() });

/** The characters that open and close each mark, the outermost first. */
const ENDS: Readonly<Record<string, string>> = { bold: "**", italic: "_", strike: "~~" };

/** The marks Markdown writes, the outermost first. */
const MARKS = ["link", ...Object.keys(ENDS)];

/** A letter or digit, which a mark's characters join rather than open or close. */
const WORD = /[\p{L}\p{N}]/u;

/** A letter or digit with another character after, as a lone one may be written by its number. */
const START_OF_WORD = /^[\p{L}\p{N}]./su;

/** The characters of a text that Markdown may read as more than themselves. */
const SPECIAL = /[\\`*_~[\]<&#]|!(?=\[)|(?<=https?|ftp|mailto|xmpp):|(?<=www)\.|(?<=\S)@(?=\S)/giu;

/** The start of a line that Markdown may read as a block's. */
const START = /^(\s*)(\d{1,9}(?=[.)])|(?=[>+=|-]))/gmu;

/** A stretch of text with the same marks, the space around it apart. */
interface Run {
    /** The text. */
    readonly text: string;
    /** Whether it is code. */
    readonly isCode: boolean;
    /** Its marks but code, by what tells them apart. */
    readonly marks: ReadonlyMap<string, { readonly type: string; readonly value: JsonValue }>;
}

/** Writes text with marks as Markdown, a mark staying open over the stretches that share it. */
export class Writer {
    /** The Markdown so far. */
    #text = "";
    /** Whether a mark closed that a letter right after would open again. */
    #isClosed = false;
    /** The marks that are open, the innermost last. */
    readonly #open: string[] = [];

    /** Write spans, a mention as its labelled link and any other inline node as nothing. */
    static write(
        spans: readonly TextSpan[],
        label: (subject: Subject) => string = (subject) => subject.id,
    ): string {
        // write each mention as the text of a link to its subject
        const linked = spans.map((span): TextSpan => {
            const subject = "node" in span ? Mention.of(span.node) : undefined;

            return subject === undefined
                ? span
                : {
                      text: `@${label(subject)}`,
                      marks: { link: { href: Mention.address(subject) } },
                  };
        });

        // close what each stretch does not share, and open what it adds, the longest first,
        //  but none at a space
        const writer = new Writer();
        const runs = runsOf(linked);
        for (const [index, run] of runs.entries()) {
            const shared = writer.#open.findIndex((key) => !run.marks.has(key));
            writer.#close(shared === -1 ? writer.#open.length : shared, runs[index - 1]);
            const lasting = (key: string): number => {
                const end = runs.findIndex((each, at) => at > index && !each.marks.has(key));

                return end === -1 ? runs.length : end;
            };
            const added = [...run.marks.keys()]
                .filter((key) => !isSpace(run) && !writer.#open.includes(key))
                .toSorted((one, other) => lasting(other) - lasting(one));
            writer.#run(run, added);
        }
        writer.#close(0, runs.at(-1));

        return writer.#text;
    }

    /** Write a stretch within the marks it opens. */
    #run(run: Run, added: readonly string[]): void {
        // write an address that stands for itself alone
        const [key] = added;
        const only = added.length === 1 && key !== undefined ? run.marks.get(key) : undefined;
        const href = only?.type === "link" ? Link.safeParse(only.value).data?.href : undefined;
        const written = run.isCode ? codeOf(run.text) : escaped(run.text);
        const isFirst = this.#open.length === 0 && run.marks.size === 1;
        if (
            isFirst &&
            !run.isCode &&
            href === run.text &&
            /^[a-z][a-z0-9+.-]+:[^\s<>]+$/iu.test(href)
        ) {
            this.#add(`<${href}>`);

            return;
        }

        // open its marks, a letter before them apart
        for (const [index, each] of added.entries()) {
            const type = run.marks.get(each)?.type ?? "";
            const isLast = index === added.length - 1;
            const isWord = isLast && START_OF_WORD.test(written);
            if (type === "link") {
                this.#text = this.#text.replace(/!$/u, "\\!");
                this.#add("[");
            } else {
                if (type === "italic" || !isWord) {
                    this.#text = this.#text.replace(/[\p{L}\p{N}]$/u, encoded);
                }
                this.#add(ENDS[type] ?? "");
            }
            this.#open.push(each);
        }
        this.#add(written);
    }

    /** Close the marks above a depth, those of the last stretch. */
    #close(depth: number, last: Run | undefined): void {
        while (this.#open.length > depth) {
            const mark = last?.marks.get(this.#open.pop() ?? "");
            const isWord = WORD.test(this.#text.at(-1) ?? "");
            const href = Link.safeParse(mark?.value).data?.href ?? "";
            this.#text +=
                mark?.type === "link" ? `](${addressOf(href)})` : (ENDS[mark?.type ?? ""] ?? "");
            this.#isClosed = mark?.type !== "link" && (mark?.type === "italic" || !isWord);
        }
    }

    /** Add Markdown, a letter right after a mark that closed apart from it. */
    #add(written: string): void {
        this.#text += this.#isClosed ? written.replace(/^[\p{L}\p{N}]/u, encoded) : written;
        this.#isClosed = false;
    }
}

/** Write an address, within brackets where it has any or a space. */
export function addressOf(href: string): string {
    const safe = href.replace(/[<>\n]/gu, encodeURIComponent);

    return /[\s()]/u.test(safe) || safe === "" ? `<${safe}>` : safe;
}

/** Split spans into stretches, the space between two with the marks both have. */
function runsOf(spans: readonly TextSpan[]): Run[] {
    // put what has the same marks together
    const joined: Run[] = [];
    for (const span of spans) {
        if (!("text" in span)) {
            continue;
        }
        const marks = new Map(
            MARKS.filter((type) => span.marks?.[type] !== undefined).map((type) => {
                const value = span.marks?.[type] ?? null;

                return [`${type} ${canonicalize(value)}`, { type, value }] as const;
            }),
        );
        const isCode = span.marks?.["code"] !== undefined;
        const last = joined.at(-1);
        const isSame =
            last?.isCode === isCode &&
            last.marks.size === marks.size &&
            [...marks.keys()].every((key) => last.marks.has(key));
        if (isSame) {
            joined[joined.length - 1] = { ...last, text: last.text + span.text };
        } else {
            joined.push({ text: span.text, isCode, marks });
        }
    }

    // take the space around a text apart, but within code
    const runs = joined.flatMap((run) => {
        const pieces = run.isCode ? [run.text] : run.text.match(/^\s+|\S(?:[\s\S]*\S)?|\s+$/gu);

        return (pieces ?? []).map((text) => ({ ...run, text }));
    });

    // mark the space as both sides of it are
    return runs.map((run, index) => {
        const sides = [
            runs.slice(0, index).findLast((each) => !isSpace(each)),
            runs.slice(index).find((each) => !isSpace(each)),
        ];
        const marks = [...run.marks].filter(([key]) =>
            sides.every((side) => side?.marks.has(key) === true),
        );

        return isSpace(run) ? { ...run, marks: new Map(marks) } : run;
    });
}

/** Report whether a stretch is space, which no mark opens or closes at. */
function isSpace({ text, isCode }: Run): boolean {
    return !isCode && text.trim() === "";
}

/** Write code between more backticks than it has, apart from its own at its ends. */
function codeOf(text: string): string {
    // put a space beside a backtick at an end, and beside space that would be taken off
    const longest = Math.max(0, ...Array.from(text.matchAll(/`+/gu), ([run]) => run.length));
    const fence = "`".repeat(longest + 1);
    const isPadded = /^`|`$|^ [\s\S]*[^ ][\s\S]* $/u.test(text);

    return isPadded ? `${fence} ${text} ${fence}` : fence + text + fence;
}

/** Escape the characters of a text that Markdown may read as more than themselves. */
function escaped(text: string): string {
    return text
        .replace(SPECIAL, (character) => `\\${character}`)
        .replace(START, (_, space: string, number: string) => `${space}${number}\\`);
}

/** Write a character by its number, as a letter that joins no mark. */
function encoded(character: string): string {
    return `&#${String(character.codePointAt(0))};`;
}
