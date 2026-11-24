import { Catalog, Localization, Message } from "@destack/locale";
import type {} from "@destack/package/import-meta";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Dropzone } from "@destack/ui/dropzone";
import { flush, LocaleContext } from "@destack/view";
import { render } from "@destack/view/test";
import de from "../locale/de.json" with { type: "json" };
import swiss from "../locale/de-CH.json" with { type: "json" };
import fr from "../locale/fr.json" with { type: "json" };
import es from "../locale/es.json" with { type: "json" };
import it from "../locale/it.json" with { type: "json" };
import nl from "../locale/nl.json" with { type: "json" };
import pt from "../locale/pt.json" with { type: "json" };

/** The marks a locale isolates a value's direction with. */
const ISOLATES = /[\u2066-\u2069]/gu;

/** A phrase in a source file, up to the mark that ends it. */
const PHRASE = /(?<![\w.])t`((?:[^`$\\]|\\.|\$\{(?:[^{}]|\{[^{}]*\})*\})*)`/gu;

/** A value in a phrase. */
const VALUE = /\$\{(?:[^{}]|\{[^{}]*\})*\}/u;

/** A value whose text varies, which a phrase's source writes out. */
const VARIANT = /\b(?:plural|select|argument)\(/u;

/** The kit's sources, without its tests and examples. */
const SOURCES = import.meta.glob<string>(
    [
        "../src/**/*.{ts,tsx}",
        "!../src/**/*.{test,example,scenario}.{ts,tsx}",
        "!../src/**/*.*.test.{ts,tsx}",
    ],
    { query: "?raw", import: "default", eager: true },
);

/** The variables of a translation. */
const VARIABLE = /\$\w+/gu;

/** List the variables of each message of a catalog, by identifier. */
function variablesOf(catalog: unknown): Record<string, string[]> {
    return Object.fromEntries(
        Object.entries(Catalog.parse(catalog).messages).map(([id, text]) => [
            id,
            [...new Set(text.match(VARIABLE))].toSorted(),
        ]),
    );
}

/** Find the phrases of the kit's sources a catalog lacks. */
function missingFrom(catalog: unknown): string[] {
    // identify each plain phrase as its module does
    const { messages } = Catalog.parse(catalog);

    return Object.values(SOURCES)
        .flatMap((source) => [...source.matchAll(PHRASE)].map(([, phrase = ""]) => phrase))
        .filter((phrase) => !VARIANT.test(phrase))
        .filter((phrase) => {
            const strings = phrase.split(VALUE).map((text) => text.replaceAll("\\`", "`"));
            const parts = strings.slice(1).map(() => "");
            const message = Message.of(strings, parts, "", import.meta.destack);

            return messages[message.id] === undefined;
        });
}

test("translate every phrase of the kit into each language", () => {
    expect({
        phrases: Object.values(SOURCES).length > 100,
        missing: [de, es, fr, it, nl, pt].map(missingFrom),
    }).toEqual({ phrases: true, missing: [[], [], [], [], [], []] });
});

test("translate the same messages with the same variables into each language", () => {
    const german = variablesOf(de);
    const own = Object.keys(variablesOf(swiss));

    expect({
        es: variablesOf(es),
        fr: variablesOf(fr),
        it: variablesOf(it),
        nl: variablesOf(nl),
        pt: variablesOf(pt),
        swiss: variablesOf(swiss),
    }).toEqual({
        es: german,
        fr: german,
        it: german,
        nl: german,
        pt: german,
        swiss: Object.fromEntries(own.map((id) => [id, german[id]])),
    });
});

test("ask for files and refuse one in the person's language", () => {
    const german = Localization.of("de", [Catalog.parse(de)]);
    const { container } = render(() => (
        <LocaleContext value={german}>
            <Dropzone
                limits={{ accept: ["application/pdf", "image/*"], size: 5_000_000 }}
                onFiles={() => {}}
            />
        </LocaleContext>
    ));
    flush();
    const zone = present(container.querySelector("[data-slot=dropzone]"), "the zone");
    const event = new Event("drop", { bubbles: true, cancelable: true });
    Object.defineProperty(event, "dataTransfer", {
        value: { files: [new File(["a"], "a.txt", { type: "text/plain" })], types: ["Files"] },
    });
    zone.dispatchEvent(event);
    flush();

    expect(
        ["title", "hint", "refusal"].map((part) =>
            zone.querySelector(`[data-slot=dropzone-${part}]`)?.textContent.replace(ISOLATES, ""),
        ),
    ).toEqual([
        "Datei hierher ziehen oder auswählen",
        "PDF oder Bilder, bis zu 5 MB",
        "a.txt hat einen anderen Dateityp",
    ]);
});
