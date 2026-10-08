import type { TokenTree } from "./token.ts";

/** The first line of every generated module. */
const HEADER = "// generate with `bun run generate` from tokens.json";

/** The TypeScript modules generated from a token tree. */
export const Source = {
    /** Render the StyleX constants of every family with primitive tokens, each a token's custom property, typography tokens rendering as text styles instead. */
    constants(tree: TokenTree): string {
        const primitives = tree.families.filter((family) =>
            family.entries.some((entry) => entry.token.$type !== "typography"),
        );
        const families = primitives.map((family) => {
            const members = family.entries
                .filter((entry) => entry.token.$type !== "typography")
                .flatMap((entry) =>
                    entry
                        .members()
                        .map((member) => [
                            `    /** ${entry.token.$description} */`,
                            `    ${property(member.key)}: "var(${member.variable})",`,
                        ]),
                );

            return [
                `/** ${family.description} */`,
                `export const ${family.name} = defineConsts({`,
                ...members.flat(),
                "});",
            ].join("\n");
        });

        return (
            [HEADER, 'import { defineConsts } from "@destack/style";', ...families].join("\n\n") +
            "\n"
        );
    },

    /** Render the StyleX text styles, one per typography token. */
    text(tree: TokenTree): string {
        const family = tree.family("text");
        const styles = family.entries
            .filter((entry) => entry.token.$type === "typography")
            .map((entry) => {
                const declarations = entry.members().map((member) => {
                    const name = member.key.slice(entry.key.length);
                    const css = name.charAt(0).toLowerCase() + name.slice(1);

                    return `        ${css}: "var(${member.variable})",`;
                });

                return [
                    `    /** ${entry.token.$description} */`,
                    `    ${entry.key}: {`,
                    ...declarations,
                    "    },",
                ];
            });

        return (
            [
                HEADER,
                'import * as style from "@destack/style";',
                [
                    `/** ${family.description} */`,
                    "export const text = style.create({",
                    ...styles.flat(),
                    "});",
                ].join("\n"),
            ].join("\n\n") + "\n"
        );
    },
};

/** Quote an object key unless it is an identifier. */
function property(key: string): string {
    return /^[A-Za-z_$][\w$]*$/u.test(key) ? key : `"${key}"`;
}
