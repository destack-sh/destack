import { ContextTracker, ExternalTokenizer, type InputStream } from "@lezer/lr";
import {
    BlockComment,
    Dialect_jsx,
    incdec,
    incdecPrefix,
    insertSemi,
    JSXStartTag,
    LineComment,
    newline,
    noSemi,
    noSemiType,
    questionDot,
    spaces,
} from "./terms.ts";

/** Every character JavaScript takes as space. */
const SPACES: ReadonlySet<number> = new Set([
    9, 10, 11, 12, 13, 32, 133, 160, 5760, 8192, 8193, 8194, 8195, 8196, 8197, 8198, 8199, 8200,
    8201, 8202, 8232, 8233, 8239, 8287, 12288,
]);

/** The characters the tokenizers look for. */
const CODES = {
    plus: 43,
    comma: 44,
    minus: 45,
    point: 46,
    slash: 47,
    star: 42,
    semicolon: 59,
    less: 60,
    question: 63,
    bracket: 91,
    brace: 125,
    end: -1,
} as const;

/** What follows a type parameter that is no element. */
const EXTENDS = "extends";

/** Whether a line break came before, as after Lezer's tracker of JavaScript. */
export const trackNewline = new ContextTracker<boolean>({
    start: false,
    shift: (hasBreak, term) =>
        term === LineComment || term === BlockComment || term === spaces
            ? hasBreak
            : term === newline,
    strict: false,
});

/** Reads the semicolon left out at a line's end. */
export const insertSemicolon = new ExternalTokenizer(
    (input, stack) => {
        if (input.next === CODES.brace || input.next === CODES.end || stack.context === true) {
            input.acceptToken(insertSemi);
        }
    },
    { contextual: true, fallback: true },
);

/** Reads that no semicolon was left out. */
export const noSemicolon = new ExternalTokenizer(
    (input, stack) => {
        // leave space and comments, which a line break may be in
        const { next } = input;
        const after = input.peek(1);
        const isComment = next === CODES.slash && (after === CODES.slash || after === CODES.star);
        const isEnd = next === CODES.brace || next === CODES.semicolon || next === CODES.end;
        if (!SPACES.has(next) && !isComment && !isEnd && stack.context !== true) {
            input.acceptToken(noSemi);
        }
    },
    { contextual: true },
);

/** Reads that no semicolon was left out before an array type's bracket. */
export const noSemicolonType = new ExternalTokenizer(
    (input, stack) => {
        if (input.next === CODES.bracket && stack.context !== true) {
            input.acceptToken(noSemiType);
        }
    },
    { contextual: true },
);

/** Reads `++`, `--` and `?.`, whose meaning depends on what is around. */
export const operatorToken = new ExternalTokenizer(
    (input, stack) => {
        // take the same mark twice
        const { next } = input;
        if (next === CODES.plus || next === CODES.minus) {
            input.advance();
            if (input.next === next) {
                input.advance();
                const isPostfix = stack.context !== true && stack.canShift(incdec);
                input.acceptToken(isPostfix ? incdec : incdecPrefix);
            }
        }
        // take an optional chain, which no digit follows
        else if (next === CODES.question && input.peek(1) === CODES.point) {
            input.advance();
            input.advance();
            if (input.next < 48 || input.next > 57) {
                input.acceptToken(questionDot);
            }
        }
    },
    { contextual: true },
);

/** Reads the start of an element, unless a type parameter starts there. */
export const jsx = new ExternalTokenizer((input, stack) => {
    // leave what is no element
    if (input.next !== CODES.less || !stack.dialectEnabled(Dialect_jsx)) {
        return;
    }
    input.advance();
    if (input.peek(0) === CODES.slash) {
        return;
    }

    // look past a word for a comma or a bound, which only a type parameter has
    const start = input.pos;
    skipSpaces(input);
    if (isWord(input.next, true)) {
        input.advance();
        while (isWord(input.next, false)) {
            input.advance();
        }
        skipSpaces(input);
        if (input.peek(0) === CODES.comma) {
            return;
        }
        let matched = 0;
        while (matched < EXTENDS.length && input.next === EXTENDS.codePointAt(matched)) {
            input.advance();
            matched += 1;
        }
        if (matched === EXTENDS.length && !isWord(input.next, false)) {
            return;
        }
    }
    input.acceptToken(JSXStartTag, start - input.pos);
});

/** Step over space. */
function skipSpaces(input: InputStream): void {
    while (SPACES.has(input.next)) {
        input.advance();
    }
}

/** Report whether a character is of a word, its first one no digit. */
function isWord(code: number, isFirst: boolean): boolean {
    return (
        (code >= 65 && code <= 90) ||
        (code >= 97 && code <= 122) ||
        code === 95 ||
        code >= 192 ||
        (!isFirst && code >= 48 && code <= 57)
    );
}
