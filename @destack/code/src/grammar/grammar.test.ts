import { expect, test } from "@destack/test";
import { Language, type LanguageName, Source, type Syntax } from "../index.ts";
import { Random } from "../test/random.ts";

/** Write the tokens of each line, plain ones as they are. */
function write(syntax: Syntax): string[] {
    const tokens = syntax.tokens();

    return Array.from({ length: syntax.source.lines }, (_, line) =>
        tokens
            .line(line)
            .map(({ kind, text }) => (kind === "plain" ? text : `${kind}\u2039${text}\u203a`))
            .join(""),
    );
}

/** Write the tokens of code in a language. */
async function tokensOf(name: LanguageName, source: string): Promise<string[]> {
    return write((await Language.of(name).load()).parse(Source.of(source)));
}

test("tell types, functions, properties, elements and values of TSX apart", async () => {
    expect(
        await tokensOf(
            "tsx",
            [
                "/** Doc",
                "    over two lines. */",
                "export class Plan<T> extends Base {",
                "    readonly #size = 3;",
                "    load(id: string) { return this.find(id) ?? null; }",
                "}",
                'const view = <Button kind="a" onClick={() => save(1)}>{plan.title} left</Button>;',
                "user.name = `n ${count + 1}`;",
            ].join("\n"),
        ),
    ).toEqual([
        "comment‹/** Doc›",
        "comment‹    over two lines. */›",
        "keyword‹export› keyword‹class› type‹Plan›punctuation‹<›type‹T›punctuation‹>› keyword‹extends› type‹Base› punctuation‹{›",
        "    keyword‹readonly› property‹#size› operator‹=› number‹3›punctuation‹;›",
        "    function‹load›punctuation‹(›variable‹id›punctuation‹:› type‹string›punctuation‹)› punctuation‹{› keyword‹return› constant‹this›punctuation‹.›function‹find›punctuation‹(›variable‹id›punctuation‹)› operator‹??› constant‹null›punctuation‹;› punctuation‹}›",
        "punctuation‹}›",
        'keyword‹const› variable‹view› operator‹=› punctuation‹<›tag‹Button› attribute‹kind›operator‹=›string‹"a"› attribute‹onClick›operator‹=›punctuation‹{()› punctuation‹=>› function‹save›punctuation‹(›number‹1›punctuation‹)}>{›variable‹plan›punctuation‹.›property‹title›punctuation‹}› leftpunctuation‹</›tag‹Button›punctuation‹>;›',
        "variable‹user›punctuation‹.›property‹name› operator‹=› string‹`n ›punctuation‹${›variable‹count› operator‹+› number‹1›punctuation‹}›string‹`›punctuation‹;›",
    ]);
});

test("read the other languages of a package, the code of a block within Markdown", async () => {
    expect({
        css: await tokensOf("css", ".card > a { color: #ff0; margin: 2px }"),
        html: await tokensOf("html", '<p class="x">Hi</p><style>a { top: 0 }</style>'),
        json: await tokensOf("json", '{ "a": [1, true, "x"] }'),
        markdown: await tokensOf("markdown", "# Plans\n\n**now** `x`\n\n```ts\nlet a = 1;\n```"),
        sql: await tokensOf("sql", "select title from tasks where id = 'a' -- one"),
        bash: await tokensOf("bash", 'echo "$HOME" # print'),
        diff: await tokensOf("diff", "-old\n+new"),
    }).toEqual({
        css: [
            ".type‹card› operator‹>› tag‹a› punctuation‹{› property‹color›punctuation‹:› constant‹#ff0›punctuation‹;› property‹margin›punctuation‹:› number‹2px› punctuation‹}›",
        ],
        html: [
            'punctuation‹<›tag‹p› attribute‹class›operator‹=›string‹"x"›punctuation‹>›Hipunctuation‹</›tag‹p›punctuation‹><›tag‹style›punctuation‹>›tag‹a› punctuation‹{› property‹top›punctuation‹:› number‹0› punctuation‹}</›tag‹style›punctuation‹>›',
        ],
        json: [
            'punctuation‹{› property‹"a"›punctuation‹:› punctuation‹[›number‹1›punctuation‹,› constant‹true›punctuation‹,› string‹"x"›punctuation‹]› punctuation‹}›',
        ],
        markdown: [
            "punctuation‹#›heading‹ Plans›",
            "",
            "punctuation‹**›strong‹now›punctuation‹**› punctuation‹`›string‹x›punctuation‹`›",
            "",
            "punctuation‹```›variable‹ts›",
            "keyword‹let› variable‹a› operator‹=› number‹1›punctuation‹;›",
            "punctuation‹```›",
        ],
        sql: [
            "keyword‹select› title keyword‹from› tasks keyword‹where› keyword‹id› operator‹=› string‹'a'› comment‹-- one›",
        ],
        bash: ['function‹echo› string‹"›variable‹$HOME›string‹"› comment‹# print›'],
        diff: ["deleted‹-old›", "inserted‹+new›"],
    });
});

test("keep every character in every language", async () => {
    const source = Source.of("// a <b> \"c\"\n\n\tfn d('e') { return 1; } # f\n@x $y %z\n");
    const changed = await Promise.all(
        Language.all.map(async (language) => {
            const grammar = await language.load();
            const read = [...grammar.parse(source).tokens()].map(({ text }) => text).join("");

            return read === source.text && grammar.language === language ? [] : [language.name];
        }),
    );

    expect(changed.flat()).toEqual([]);
});

test("read the same tokens of a range, after edits and after parsing on as of the whole", async () => {
    // type words into code of a parsed and of a streamed language
    const code = {
        tsx: "export function view(a: number) {\n    return <p title={`n ${a}`}>hi</p>;\n}\n",
        sql: "select a, count(b) from t where c = 'd' -- e\norder by a;\n",
    };
    const random = new Random(5);
    const wrong = await Promise.all(
        (["tsx", "sql"] as const).map(async (name) => {
            const grammar = await Language.of(name).load();
            const text = code[name].repeat(60);
            let syntax = grammar.parse(Source.of(text), 300);
            const first = syntax.tokens({ from: 0, to: 200 });
            syntax = syntax.parse();
            const isFirst =
                JSON.stringify([...first]) ===
                JSON.stringify([...syntax.tokens().within({ from: 0, to: 200 })]);
            const places = Array.from({ length: 40 }, () => {
                // type within a word, then compare a screen and the whole with a new parse
                const { text: now } = syntax.source;
                const at = now.indexOf("count", random.below(now.length / 2));
                const place = at === -1 ? now.indexOf("view") + 1 : at + 1;
                syntax = syntax.edit({ from: place, to: place, insert: "q" });
                const whole = grammar.parse(syntax.source).tokens();
                const from = syntax.source.start(random.below(60));
                const range = { from, to: from + 120 };

                return (
                    syntax.tokens().ends.join() === whole.ends.join() &&
                    syntax.tokens().kinds.join() === whole.kinds.join() &&
                    JSON.stringify([...syntax.tokens(range)]) ===
                        JSON.stringify([...whole.within(range)])
                );
            });

            return [isFirst, places.every(Boolean), syntax.parse().parsed];
        }),
    );

    expect(wrong).toEqual([
        [true, true, code.tsx.length * 60 + 40],
        [true, true, code.sql.length * 60 + 40],
    ]);
});
