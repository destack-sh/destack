# @destack/locale

Localise messages and format numbers, money and dates for a person's language and region.

## Overview

`@destack/locale` is Lingui's `t` macro over Unicode MessageFormat 2: messages as JSON, catalogs, and per-reader rendering and formatting.

Packages write messages in their source language:

- `t`: writes a `Message` from a template, as Lingui's `t` does.
- `argument`, `plural` and `select`: a supplied value, text by plural category, and text by case.
- `verbatim`: text no catalog translates, such as a brand.
- `Message`: a message as JSON.

Translations ship with the package:

- `Catalog`: one package's translations into one locale, in `locale/<tag>.json`.

Readers render in their locale:

- `Localization`: renders messages and formats numbers, money, dates and lists in a locale.
- `Locale`: reads, orders and negotiates BCP 47 language tags.

Write a message, then render it for a reader in Austria:

```ts
import { argument, Localization, plural, t } from "@destack/locale";

// write a message with a count
const archived = t`Archived ${plural(argument("count"), { one: "# page", other: "# pages" })}`;

// render it, and format a price
const localization = Localization.of("de-AT", []);
const text = localization.render(archived, { count: 3 });
const price = localization.money(1200, "EUR");
```

## Reference

### t

`t` writes a `Message` from a template, tagged with the module's package.
`Message.context(context)` gives the same text separate identifiers, such as a button's "Save" and a menu's.

| Part                               | Becomes                                                                           |
| ---------------------------------- | --------------------------------------------------------------------------------- |
| a JSON value                       | a positional variable, `p0`, `p1`, …, its value kept in the message               |
| `argument(name)`                   | a variable of a camel case name, which the reader supplies when rendering         |
| `plural(value, { one, other, … })` | a text by the value's CLDR plural category, `#` standing for the value            |
| `select(value, { …, other })`      | a text by the value, `other` covering every unlisted one                          |
| `` verbatim`…` ``                  | a plain string instead of a message, which no catalog translates, such as a brand |

### Message

A `Message` is JSON, rendered later by each reader.

| Field     | What it is                                                          |
| --------- | ------------------------------------------------------------------- |
| `package` | the package that wrote it, whose catalogs translate it              |
| `id`      | a hash of its source and context, which catalogs translate it under |
| `source`  | the source text in MessageFormat 2                                  |
| `values`  | the values of its positional variables                              |

### Catalog

A catalog is a JSON file in a package's `locale` directory, `locale/<tag>.json` for its own messages and `locale/<package>/<tag>.json` for a dependency's.
`Catalog.read(reader)` reads every catalog a build ships.

| Field      | What it is                                                     |
| ---------- | -------------------------------------------------------------- |
| `package`  | the package whose messages it translates                       |
| `locale`   | the locale it translates into, such as `de`                    |
| `messages` | the translated MessageFormat 2 source of each message, by `id` |
| `drafts`   | the identifiers of machine translations no person reviewed yet |

### Localization

`Localization.of(tag, catalogs, { source, direction, request })` binds a locale to catalogs.
It renders in the nearest translated locale, else in the `source` language, English by default.
`request` loads a missing package's catalogs.

| Verb                             | What it does                                                                                 |
| -------------------------------- | -------------------------------------------------------------------------------------------- |
| `render(message, arguments)`     | renders a message with the arguments its reader supplies                                     |
| `number(value, options)`         | formats a number, as `Intl.NumberFormat`                                                     |
| `money(amount, currency)`        | formats minor units, such as `1200` `"EUR"` as `€12.00`, and `"x-credits"` as `1200 credits` |
| `date(time, options)`            | formats an instant in epoch milliseconds, as `Intl.DateTimeFormat`                           |
| `dateRange(start, end, options)` | formats the span between two instants, sharing their common parts                            |
| `list(items, options)`           | joins items, such as `Ada, Grace, and Linus`                                                 |
| `relative(time, now)`            | formats an instant in its largest whole unit, such as `3 days ago`                           |
| `tag`, `direction`               | the locale, and the direction its text runs in                                               |

### Locale

`Locale` reads language tags, and `LocaleTag`, `Country` and `Direction` are their schemas.

| Verb                                     | What it does                                                                      |
| ---------------------------------------- | --------------------------------------------------------------------------------- |
| `Locale.parse(tag)`                      | reads a tag in its canonical casing, such as `en-GB`, refusing one that is no tag |
| `Locale.accepted(header)`                | reads the tags of an `Accept-Language` header, most preferred first               |
| `Locale.negotiate(preferred, available)` | picks the available tag nearest to a person's preferred ones, as RFC 4647 lookup  |
| `Locale.fallback(tag, source)`           | lists the tags a message is read in, through CLDR's parents to the source         |
| `Locale.direction(tag)`                  | reads `"ltr"` or `"rtl"` from the tag's likely script                             |

## Examples

### Render a notification in each reader's language

Each reader's `Localization` renders a stored message.

```ts
import { type Catalog, Locale, Localization, type Message } from "@destack/locale";

/** Render a stored message in the language a request asks for. */
export function renderFor(
    request: Request,
    message: Message,
    catalogs: readonly Catalog[],
): string {
    // pick the nearest language
    const header = request.headers.get("accept-language") ?? "";
    const available = ["en", ...catalogs.map((catalog) => catalog.locale)];
    const tag = Locale.negotiate(Locale.accepted(header), available) ?? "en";

    return Localization.of(tag, catalogs).render(message);
}
```

### Choose words by case and count

`select` picks text by a value, and `plural` by a count's plural category.

```ts
import { argument, plural, select, t } from "@destack/locale";

// say who shared how many notes, as each language words it
export const shared = t`${select(argument("pronoun"), { woman: "She", man: "He", other: "They" })} shared ${plural(argument("count"), { one: "# note", other: "# notes" })}`;
```

### Tell two messages with one text apart

`Message.context` gives the same text separate identifiers, so a button's "Save" and a menu's can translate differently.

```ts
import { Message } from "@destack/locale";

// title the save button under its own context
export const save = Message.context("button")`Save`;
```

### Format values for a reader

A `Localization` formats numbers, money in minor units, dates, ranges, lists and relative times.

```ts
import { Localization } from "@destack/locale";

/** Describe an invoice for a German reader. */
export function describeInvoice(total: number, due: number, now: number): string {
    // format the amount and the due date, relative to now
    const german = Localization.of("de", []);
    const amount = german.money(total, "EUR");

    return `${amount}, ${german.relative(due, now)}`;
}
```

### Translate a message

A catalog translates each message by its identifier, in MessageFormat 2 with the same variables.
This is `locale/de.json` for the overview's `archived` message, as a draft.

```json
{
    "package": "package-01a0e95b-c8db-7258-a257-e7661dbc93c3",
    "locale": "de",
    "messages": {
        "g5cu7a6oiuz6": ".input {$count :number}\n.match $count\none {{{$count} Seite archiviert}}\n* {{{$count} Seiten archiviert}}"
    },
    "drafts": ["g5cu7a6oiuz6"]
}
```

## Comparison

### Lingui

`t`, `plural` and `select` basically follow Lingui's macros, with messages in MessageFormat 2 instead of ICU MessageFormat.
Messages are JSON, rendered by each reader.

| Lingui                         | `@destack/locale`                 | Difference                                                      |
| ------------------------------ | --------------------------------- | --------------------------------------------------------------- |
| `` t`…` ``                     | `` t`…` ``                        | Writes a `Message` as JSON instead of the rendered text.        |
| `plural`, `select`             | the same                          | The same.                                                       |
| `t({ message, context })`      | `` Message.context(context)`…` `` | A tag of its own for each context.                              |
| ICU MessageFormat              | MessageFormat 2                   | Rendered by the `messageformat` library.                        |
| `locales/<locale>/messages.po` | `locale/<tag>.json`               | JSON, with machine translations marked as `drafts`.             |
| `i18n.number`, `i18n.date`     | `Localization.number`, `date`, …  | Adds `money` in minor units and `relative` in the largest unit. |
