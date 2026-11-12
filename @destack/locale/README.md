# @destack/locale

Localise messages and format numbers, money and dates for a person's language and region.

## Overview

A package writes its messages with `t`, and each reader's localization renders them in their language later, such as a notification in each recipient's.

- **Message**: text of its package in Unicode MessageFormat 2, with `plural`, `select` and named arguments; `t` writes one.
- **Catalog**: one package's translations into one locale, read from its `locale/<tag>.json`.
- **Localization**: a locale bound to its catalogs; `render`, `number`, `money`, `date`, `list` and `relative`.
- **Locale**: BCP 47 tags with CLDR parent locales; `fallback`, `accepted` and `negotiate`.

```ts
import { argument, Localization, plural, t } from "@destack/locale";

const archived = t`Archived ${plural(argument("count"), { one: "# page", other: "# pages" })}`;
const localization = Localization.of("de-AT", []);
const text = localization.render(archived, { count: 3 });
const price = localization.money(1200, "EUR"); // "€ 12,00"
```

## Examples

### Format dates and times

`relative` picks the largest whole unit, where `Intl.RelativeTimeFormat` takes one.

```ts
import { Localization } from "@destack/locale";

const localization = Localization.of("en-GB", []);
const now = Date.now();
localization.relative(now - 3 * 86_400_000, now); // "3 days ago"
localization.date(now, { dateStyle: "medium" });
```

### Pick a locale

`Locale.accepted` reads an `Accept-Language` header, and `Locale.fallback` lists a tag's CLDR parents.

```ts
import { Locale } from "@destack/locale";

Locale.accepted("fr-CH, de;q=0.7, *;q=0.5"); // ["fr-CH", "de"]
Locale.fallback("es-MX", "en"); // ["es-MX", "es-419", "es", "en"]
```

### Keep a word untranslated

`verbatim` marks text left out of extraction and catalogs, such as a product name.

```ts
import { verbatim } from "@destack/locale";

const brand = verbatim`Destack`;
```

## Comparison

### Lingui

`t`, `plural` and `select` follow Lingui's macros; three things differ.

- Messages are Unicode MessageFormat 2, rendered by `messageformat`.
- A message is data of its package, so a server writes it once and each reader renders it in their language.
- `money` takes minor units and knows custom currencies such as credits.
