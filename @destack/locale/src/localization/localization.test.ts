import { ModuleMetadata } from "@destack/package";
import { expect, test } from "@destack/test";
import type { Catalog } from "../message/catalog.ts";
import { Message, plural, select, t } from "../message/message.ts";
import { Localization } from "./localization.ts";

/** The first strong isolate MessageFormat 2 places around a string value. */
const OPEN = String.fromCodePoint(0x2068);

/** The pop directional isolate closing a string value. */
const CLOSE = String.fromCodePoint(0x2069);

/** A fixed instant to measure relative times from. */
const NOW = Date.UTC(2026, 9, 4, 12);

/** One hour in milliseconds. */
const HOUR = 60 * 60 * 1000;

/** A module of the notes package. */
const NOTES = ModuleMetadata.parse({
    package: {
        id: "package-01996ab0-0000-7000-8000-000000000001",
        name: "@example/notes",
        version: "2026.10.0",
    },
});

/** A module of the mail package. */
const MAIL = ModuleMetadata.parse({
    package: {
        id: "package-01996ab0-0000-7000-8000-000000000002",
        name: "@example/mail",
        version: "2026.10.0",
    },
});

/** Write the notes package's plural message about archiving notes into a notebook. */
function archived(count: number): Message {
    return Message.context(
        "",
        NOTES,
    )`Archived ${plural(count, { one: "# note", other: "# notes" })} in ${"Trips"}`;
}

/** Write the message naming who replied, selected by gender. */
function replied(person: string): Message {
    return t`${select(person, { woman: "She", man: "He", other: "They" })} replied`;
}

/** The notes package's German catalog translating the archive message. */
const GERMAN: Catalog = {
    package: NOTES.package.id,
    locale: "de",
    messages: {
        [archived(1).id]: [
            ".input {$p0 :number}",
            ".match $p0",
            "one {{{$p0} Notiz in {$p1} archiviert}}",
            "* {{{$p0} Notizen in {$p1} archiviert}}",
        ].join("\n"),
    },
};

test("render a message in the nearest translation along the reader's chain", () => {
    expect([
        Localization.of("de-AT", [GERMAN]).render(archived(1)),
        Localization.of("de", [GERMAN]).render(archived(3)),
    ]).toEqual([
        `1 Notiz in ${OPEN}Trips${CLOSE} archiviert`,
        `3 Notizen in ${OPEN}Trips${CLOSE} archiviert`,
    ]);
});

test("render a message no catalog translates in its source text and language", () => {
    expect([
        Localization.of("en-GB", [GERMAN]).render(archived(1)),
        Localization.of("fr", [GERMAN]).render(archived(2)),
        Localization.of("de", [GERMAN]).render(Message.context("", NOTES)`Saved`),
    ]).toEqual([
        `Archived 1 note in ${OPEN}Trips${CLOSE}`,
        `Archived 2 notes in ${OPEN}Trips${CLOSE}`,
        "Saved",
    ]);
});

test("translate the same source text of two packages by each package's own catalogs", () => {
    // write "Close" in both packages, each translated differently into German
    const notes = Message.context("", NOTES)`Close`;
    const mail = Message.context("", MAIL)`Close`;
    const catalogs: Catalog[] = [
        { package: NOTES.package.id, locale: "de", messages: { [notes.id]: "Schließen" } },
        { package: MAIL.package.id, locale: "de", messages: { [mail.id]: "Zumachen" } },
    ];
    const german = Localization.of("de", catalogs);

    expect([notes.id === mail.id, german.render(notes), german.render(mail)]).toEqual([
        true,
        "Schließen",
        "Zumachen",
    ]);
});

test("select a variant by value with the catch-all covering unlisted values", () => {
    const english = Localization.of("en", []);
    expect([
        english.render(replied("woman")),
        english.render(replied("man")),
        english.render(replied("someone")),
    ]).toEqual(["She replied", "He replied", "They replied"]);
});

test("write money from minor units by each currency's digits, and custom currencies by their code", () => {
    const [english, german] = [Localization.of("en", []), Localization.of("de", [])];
    expect([
        english.money(1200, "EUR"),
        german.money(1200, "EUR"),
        english.money(1200, "JPY"),
        english.money(1200, "KWD"),
        english.money(1200, "x-credits"),
        english.money(0, "USD"),
    ]).toEqual(["€12.00", "12,00 €", "¥1,200", "KWD 1.200", "1,200 credits", "$0.00"]);
});

test("write numbers, dates, date ranges and lists as each locale does", () => {
    const [english, german] = [Localization.of("en", []), Localization.of("de", [])];
    expect([
        english.number(1234.5),
        german.number(1234.5),
        english.date(NOW, { dateStyle: "long", timeZone: "UTC" }),
        german.date(NOW, { dateStyle: "long", timeZone: "UTC" }),
        english.dateRange(NOW, NOW + 3 * 24 * HOUR, { dateStyle: "medium", timeZone: "UTC" }),
        german.dateRange(NOW, NOW + 3 * 24 * HOUR, { dateStyle: "medium", timeZone: "UTC" }),
        english.list(["notes", "tasks", "pages"]),
        german.list(["Notizen", "Aufgaben"]),
    ]).toEqual([
        "1,234.5",
        "1.234,5",
        "October 4, 2026",
        "4. Oktober 2026",
        "Oct 4 – 7, 2026",
        "04.–07.10.2026",
        "notes, tasks, and pages",
        "Notizen und Aufgaben",
    ]);
});

test("write an instant relative to now in its largest whole unit, past and future", () => {
    const [english, german] = [Localization.of("en", []), Localization.of("de", [])];
    expect([
        english.relative(NOW - 3 * 24 * HOUR, NOW),
        german.relative(NOW + 2 * HOUR, NOW),
        english.relative(NOW - 24 * HOUR, NOW),
        english.relative(NOW - 10_000, NOW),
        english.relative(NOW, NOW),
    ]).toEqual(["3 days ago", "in 2 Stunden", "yesterday", "10 seconds ago", "now"]);
});
