import type { Token, TokenKind } from "@destack/code";
import * as style from "@destack/style";
import { code } from "@destack/theme/code";
import { createMemo, type JSX } from "@destack/view";

/** The properties of some tokens. */
export interface CodeTokensProperties {
    /** The tokens, in order. */
    readonly tokens: readonly Token[];
}

/** The look of each kind of token, none for the kinds in the color around them. */
const LOOKS: Readonly<Partial<Record<TokenKind, style.Styles>>> = code;

/** Render the tokens of a line of code, each in its kind's color. */
export function CodeTokens(properties: CodeTokensProperties): JSX.Element {
    // join the tokens without a look, which need no element
    const pieces = createMemo(() => {
        const joined: Token[] = [];
        for (const token of properties.tokens) {
            const last = joined.at(-1);
            if (last?.kind === "plain" && LOOKS[token.kind] === undefined) {
                joined[joined.length - 1] = {
                    ...last,
                    text: last.text + token.text,
                    to: token.to,
                };
            } else {
                const kind = LOOKS[token.kind] === undefined ? "plain" : token.kind;
                joined.push({ ...token, kind });
            }
        }

        return joined;
    });

    return (
        <>
            {pieces().map(({ kind, text: piece }) => {
                const look = LOOKS[kind];

                return look === undefined ? (
                    piece
                ) : (
                    <span data-token={kind} {...style.attrs(look)}>
                        {piece}
                    </span>
                );
            })}
        </>
    );
}
