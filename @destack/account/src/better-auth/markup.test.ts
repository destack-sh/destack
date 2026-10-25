import { Digest } from "@destack/schema";
import { testSimulation } from "@destack/object/test";
import { expect, test } from "@destack/test";
import { html, render } from "./markup.ts";

test("allow the one script a page runs by its SHA-256 in the page's content security policy", async () => {
    // render a page, and hash the script it runs
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("page");
    const response = await render("Signed in", html`<p>Welcome</p>`);
    const [, script] = /<script>(.*)<\/script>/su.exec(await response.text()) ?? [];
    const digest = await Digest.of(script ?? "", crypto);

    // allow that script alone, and nothing else
    expect(response.headers.get("content-security-policy")).toBe(
        `default-src 'none'; script-src 'sha256-${Uint8Array.fromHex(digest).toBase64()}'; style-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'`,
    );
});
