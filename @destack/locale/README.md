# @destack/locale

`t`, `plural` and `select` are Lingui's macros writing Unicode MessageFormat 2 through `messageformat`, `Localization` is `Intl`'s formatters bound to a locale and its catalogs, and `Locale` is BCP 47 tags with CLDR parent locales and RFC 4647 lookup.

```ts
// Lingui's msg descriptor as JSON of its package, rendered later
t`Archived ${plural(count, { one: "# page", other: "# pages" })} in ${space.name}`;
Message.context("button")`Open`; // Lingui's context option as a tag
verbatim`Destack`; // left out of extraction and catalogs, where Lingui leaves plain strings
localization.money(1200, "EUR"); // minor units, where Intl.NumberFormat takes major: "12,00 €"
localization.money(1200, "x-credits"); // a custom currency beside its code: "1,200 credits"
// the largest whole unit, where Intl.RelativeTimeFormat takes one
localization.relative(threeDaysAgo, Date.now());
// CLDR parents, then the source language: ["es-MX", "es-419", "es", "en"]
Locale.fallback("es-MX", "en");
Locale.accepted("fr-CH, de;q=0.7, *;q=0.5"); // an Accept-Language header by weight: ["fr-CH", "de"]
Country.parse("CH"); // an ISO 3166-1 alpha-2 country code
```

## Messages

`t` writes a message of its module's package, which the reader's locale renders later, such as a notification in each recipient's.

```ts
const archived = t`Archived ${count} pages`;
Localization.of("de-AT", catalogs).render(archived); // "3 Seiten archiviert"
```

## Catalogs

A `Catalog` translates one package's messages into one locale from the package's `locale/<tag>.json`, which its build ships and `Catalog.read` reads back.

```json
{
    "package": "package-01a1075a-34ee-7343-832f-9faf3e1c456c",
    "locale": "de",
    "messages": { "1x3k9q2…": "Schließen" },
    "drafts": ["1x3k9q2…"]
}
```
