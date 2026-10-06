# @destack/locale

Localise messages and format numbers, money and dates for a person's language and region.

## Localization

`Localization.of` binds a person's locale to the packages' catalogs, rendering messages and formatting values in it.

```ts
import { Localization } from "@destack/locale";

const german = Localization.of("de-AT", catalogs); // messages written in "en", the default source
german.tag; // "de-AT"
german.direction; // "ltr", the locale's own unless the options set another
german.render(archived); // "3 Notizen in ⁨Trips⁩ archiviert", values isolated by direction
german.number(1234.5); // "1.234,5"
german.money(1200, "EUR"); // "12,00 €"
german.date(Date.now(), { dateStyle: "long" }); // "4. Oktober 2026"
german.dateRange(start, end, { dateStyle: "medium" }); // "04.–07.10.2026"
german.list(["Notizen", "Aufgaben"]); // "Notizen und Aufgaben"
german.relative(threeDaysAgo, Date.now()); // "vor 3 Tagen"
Localization.of("en", []).money(1200, "x-credits"); // "1,200 credits", formatting without catalogs
Localization.of("en", [], { direction: "rtl" }).direction; // "rtl", such as an example mirrored for review
```

## Locales

`Locale.fallback` lists the tags a message is read in, nearest first: the tag, its CLDR parents and the package's source language.

```ts
import { Locale } from "@destack/locale";

Locale.parse("de-at"); // "de-AT"
Locale.fallback("es-MX", "en"); // ["es-MX", "es-419", "es", "en"]
Locale.negotiate(["de-AT", "en"], ["fr", "de"]); // "de", the RFC 4647 lookup
Locale.accepted("fr-CH, de;q=0.7, *;q=0.5"); // ["fr-CH", "de"], an Accept-Language header by weight
Locale.direction("ar-EG"); // "rtl", from the script the locale most likely writes in
```

## Messages

`t` writes a JSON message of its module's package in MessageFormat 2, which `Message` parses back and the reader's locale renders later, such as a notification in each recipient's.

```ts
import { Message, plural, select, t, verbatim } from "@destack/locale";

// the module transform passes the writing module to t and Message.context, which throw without it

const archived = t`Archived ${plural(count, { one: "# note", other: "# notes" })} in ${notebook.name}`;
// { package, id, source: ".input {$p0 :number}\n.match $p0\none {{Archived {$p0} note in {$p1}}}\n* {{…}}", values }
const replied = t`${select(gender, { woman: "She", man: "He", other: "They" })} replied`;
const brand = verbatim`Destack`; // never extracted or translated
const label = Message.context("button")`Open`; // told apart from another "Open" by its context
```

## Catalogs

A `Catalog` translates one package's messages into one locale from the package's `locale/<tag>.json`, which its build ships and `Catalog.read` reads back.

```ts
import { Catalog } from "@destack/locale";

const german: Catalog = {
    package: "package-01a10754-6288-731f-a2ef-8b79479a427a",
    locale: "de",
    messages: { [archived.id]: "…{$p0} Notizen in {$p1} archiviert…" },
    drafts: [], // machine-translated messages no person reviewed yet
};
const shipped = await Catalog.read(reader); // the catalogs a build ships, each its package's own
```

## Translations

A package translates its messages in `locale/<tag>.json` files beside its `destack.json`, one `Catalog` per locale, which its build validates and ships with its outputs.

```json
// locale/de.json
{
    "package": "package-01a1075a-34ee-7343-832f-9faf3e1c456c",
    "locale": "de",
    "messages": { "1x3k9q2…": "Schließen" },
    "drafts": ["1x3k9q2…"]
}
```

## Interfaces

A view or a website renders its messages through `useLocale()` from `@destack/locale/solid`, whose `Localization` the host provides in the person's locale with the catalogs of the view's package and its dependencies, and the source language without a provider.

```tsx
import { plural, t } from "@destack/locale";
import { useLocale } from "@destack/locale/solid";

export function Archived(properties: { count: number; notebook: string }) {
    const locale = useLocale(); // provided by renderView for views, by LocaleContext for websites

    return (
        <p>
            {locale.render(
                t`Archived ${plural(properties.count, { one: "# note", other: "# notes" })} in ${properties.notebook}`,
            )}
        </p>
    );
}

// a website provides it itself
<LocaleContext value={Localization.of(Locale.parse("de-AT"), catalogs)}>{page}</LocaleContext>;
```
