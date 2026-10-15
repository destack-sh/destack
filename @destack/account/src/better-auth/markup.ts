import { SOURCE_LOCALE } from "@destack/locale";
import { aligned, Digest, present } from "@destack/schema";

/** The HTML escapes of the characters markup gives meaning to. */
const ESCAPES: Readonly<Record<string, string>> = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
};

/** The alert text color, a red with 4.8:1 contrast on white, above WCAG AA's 4.5:1. */
const ALERT_COLOR = "#d92d20";

/** The stylesheet every page shares. */
const STYLE = `
:root { color-scheme: light dark; font-family: system-ui, sans-serif; line-height: 1.5; }
body { margin: 0; min-height: 100vh; display: grid; place-items: center; }
main { width: min(100% - 2rem, 24rem); }
form { display: grid; gap: 0.75rem; margin-block: 1.5rem; }
input, button { font: inherit; padding: 0.625rem 0.75rem; border-radius: 0.375rem; }
input { border: 1px solid GrayText; }
button { border: 0; background: CanvasText; color: Canvas; cursor: pointer; }
[role="alert"] { color: ${ALERT_COLOR}; }
`;

/** The one script the pages run: it fills each form's time zone field with the browser's. */
const TIME_ZONE_SCRIPT = `for (const field of document.querySelectorAll('input[name="timeZone"]')) field.value = Intl.DateTimeFormat().resolvedOptions().timeZone;`;

/** The time zones a person without scripts chooses from, the runtime's IANA zones. */
const TIME_ZONES = Intl.supportedValuesOf("timeZone");

/** The headers of every page: HTML that runs the time zone script alone and refuses framing. */
const HEADERS = Digest.of(TIME_ZONE_SCRIPT).then((digest) => {
    // admit the time zone script by its hash
    const script = `'sha256-${Uint8Array.fromHex(digest).toBase64()}'`;

    return {
        "content-type": "text/html; charset=utf-8",
        "content-security-policy": `default-src 'none'; script-src ${script}; style-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'`,
        "referrer-policy": "same-origin",
    };
});

/** HTML the pages wrote or escaped. */
export class Markup {
    /** The HTML text. */
    readonly text: string;

    /** Wrap HTML text. */
    constructor(text: string) {
        this.text = text;
    }
}

/** Build markup, and escape every interpolated text value. */
export function html(
    strings: TemplateStringsArray,
    ...values: readonly (string | Markup | readonly Markup[])[]
): Markup {
    // join the strings with the written values
    let text = aligned(strings, 0);
    for (const [index, value] of values.entries()) {
        text += write(value) + aligned(strings, index + 1);
    }

    return new Markup(text);
}

/** Write an interpolated value: markup as it is, text escaped, a list of markup joined. */
function write(value: string | Markup | readonly Markup[]): string {
    // keep markup
    if (value instanceof Markup) {
        return value.text;
    }
    // escape text
    else if (typeof value === "string") {
        return escape(value);
    }
    // join a list of markup
    else {
        return value.map((item) => item.text).join("");
    }
}

/** Escape a value for HTML text and attributes. */
function escape(value: string): string {
    return value.replace(/[&<>"']/gu, (character) =>
        present(ESCAPES[character], `the escape of ${character}`),
    );
}

/** Announce a message to the person. */
export function alert(message: string): Markup {
    return html`<p role="alert">${message}</p>`;
}

/** Write a form's time zone field. */
export function timeZoneField(): Markup {
    // offer the list to a person without scripts
    const options = TIME_ZONES.map((zone) => html`<option value="${zone}">${zone}</option>`);

    return html`<noscript>
            <label>
                Time zone
                <select name="timeZone" required>
                    <option value="">Choose your time zone</option>
                    ${options}
                </select>
            </label>
        </noscript>
        <input type="hidden" name="timeZone" />`;
}

/** Answer a page in the pages' own language. */
export async function render(
    title: string,
    main: Markup,
    status = 200,
    cookies: readonly string[] = [],
): Promise<Response> {
    // write the document
    const document = html`<!doctype html>
        <html lang="${SOURCE_LOCALE}">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>${title}</title>
                <style>
                    ${new Markup(STYLE)}
                </style>
            </head>
            <body>
                <main>
                    <h1>${title}</h1>
                    ${main}
                </main>
                ${new Markup(`<script>${TIME_ZONE_SCRIPT}</script>`)}
            </body>
        </html>`;

    // keep the cookies the routes set
    const headers = new Headers(await HEADERS);
    for (const cookie of cookies) {
        headers.append("set-cookie", cookie);
    }

    return new Response(document.text, { status, headers });
}

/** Send the browser on after a form with the cookies the routes set. */
export function redirect(location: string, cookies: readonly string[]): Response {
    // keep the cookies the routes set
    const headers = new Headers({ location });
    for (const cookie of cookies) {
        headers.append("set-cookie", cookie);
    }

    return new Response(null, { status: 303, headers });
}
