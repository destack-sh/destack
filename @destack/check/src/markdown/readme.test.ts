import { expect, test } from "@destack/test";
import { Readme } from "./readme.ts";

test("read a README's TypeScript listings in order, leaving other languages out", () => {
    const markdown = [
        "# @destack/mail",
        "",
        "```ts",
        'export const outbound = defineMail({ name: "outbound" });',
        "```",
        "",
        "```sh",
        "destack install @destack/mail",
        "```",
        "",
        "```ts no_run",
        "const smtp = new SmtpTransport(client, { from });",
        "```",
        "",
    ].join("\n");

    expect(Readme.listings(markdown)).toEqual([
        { language: "ts", body: 'export const outbound = defineMail({ name: "outbound" });\n' },
        { language: "ts", body: "const smtp = new SmtpTransport(client, { from });\n" },
    ]);
});
