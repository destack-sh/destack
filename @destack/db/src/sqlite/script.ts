/**
 * The transitions of SQLite's statement completion state machine, as `sqlite3_complete` defines them.
 *
 * States: invalid, start, normal, explain, create, trigger, semicolon within a trigger, end of a trigger.
 * Tokens: semicolon, whitespace, other, EXPLAIN, CREATE, TEMP, TRIGGER, END.
 */
const TRANSITIONS: readonly (readonly number[])[] = [
    [1, 0, 2, 3, 4, 2, 2, 2],
    [1, 1, 2, 3, 4, 2, 2, 2],
    [1, 2, 2, 2, 2, 2, 2, 2],
    [1, 3, 3, 2, 4, 2, 2, 2],
    [1, 4, 2, 2, 2, 4, 5, 2],
    [6, 5, 5, 5, 5, 5, 5, 5],
    [6, 6, 5, 5, 5, 5, 5, 7],
    [1, 7, 5, 5, 5, 5, 5, 5],
];

/** The state after a complete statement's closing semicolon. */
const START = 1;

/** The token columns of the transitions. */
const TOKEN = {
    semicolon: 0,
    whitespace: 1,
    other: 2,
    explain: 3,
    create: 4,
    temp: 5,
    trigger: 6,
    end: 7,
} as const;

/** The keywords the state machine tells apart. */
const KEYWORDS: Readonly<Record<string, number>> = {
    explain: TOKEN.explain,
    create: TOKEN.create,
    temp: TOKEN.temp,
    temporary: TOKEN.temp,
    trigger: TOKEN.trigger,
    end: TOKEN.end,
};

/** Scripts of several SQLite statements. */
export const SqliteScript = {
    /** Split a script into complete SQLite statements with whole trigger bodies. */
    statements(script: string): string[] {
        // run the state machine and cut completed statements
        const statements: string[] = [];
        let state = 0;
        let start = 0;
        for (let index = 0; index < script.length;) {
            // read the next token and where it ends
            const { token, end } = next(script, index);

            // close a statement at the semicolon completing it
            state = TRANSITIONS[state]![token]!;
            if (token === TOKEN.semicolon && state === START) {
                statements.push(script.slice(start, end).trim());
                start = end;
            }
            index = end;
        }

        // keep a last statement without its closing semicolon
        const rest = script.slice(start).trim();
        if (rest.length > 0 && state !== 0 && state !== START) {
            statements.push(rest);
        }

        return statements;
    },
};

/** Read the token at an index and the index after it. */
function next(script: string, index: number): { readonly token: number; readonly end: number } {
    const character = script[index]!;

    // read a semicolon
    if (character === ";") {
        return { token: TOKEN.semicolon, end: index + 1 };
    }
    // read whitespace
    else if (/\s/.test(character)) {
        return { token: TOKEN.whitespace, end: index + 1 };
    }
    // read a line comment to its line end
    else if (character === "-" && script[index + 1] === "-") {
        const end = script.indexOf("\n", index);

        return { token: TOKEN.whitespace, end: end === -1 ? script.length : end + 1 };
    }
    // read a block comment to its close
    else if (character === "/" && script[index + 1] === "*") {
        const end = script.indexOf("*/", index + 2);

        return { token: TOKEN.whitespace, end: end === -1 ? script.length : end + 2 };
    }
    // read a quoted string or identifier
    else if (character === "'" || character === '"' || character === "`" || character === "[") {
        const close = character === "[" ? "]" : character;
        const end = script.indexOf(close, index + 1);

        return { token: TOKEN.other, end: end === -1 ? script.length : end + 1 };
    }
    // read a word and match its keyword
    else if (/[\w$]/.test(character)) {
        let end = index + 1;
        while (end < script.length && /[\w$]/.test(script[end]!)) {
            end += 1;
        }
        const word = script.slice(index, end).toLowerCase();

        return { token: KEYWORDS[word] ?? TOKEN.other, end };
    }

    return { token: TOKEN.other, end: index + 1 };
}
