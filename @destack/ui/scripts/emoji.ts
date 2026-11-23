import { fileURLToPath } from "node:url";
import { BunEnvironment } from "@destack/environment/bun";
import { schema } from "@destack/schema";

/** The emojibase locale of each language tag the picker covers. */
const LOCALES = {
    bn: "bn",
    da: "da",
    de: "de",
    en: "en",
    "en-GB": "en-gb",
    es: "es",
    "es-MX": "es-mx",
    et: "et",
    fi: "fi",
    fr: "fr",
    hi: "hi",
    hu: "hu",
    it: "it",
    ja: "ja",
    ko: "ko",
    lt: "lt",
    ms: "ms",
    nb: "nb",
    nl: "nl",
    pl: "pl",
    pt: "pt",
    ru: "ru",
    sv: "sv",
    th: "th",
    uk: "uk",
    vi: "vi",
    zh: "zh",
    "zh-Hant": "zh-hant",
} as const;

/** An emoji of emojibase's compact data, with the fields the picker reads. */
const CompactEmoji = schema.looseObject({
    /** The emoji's character sequence. */
    unicode: schema.string(),
    /** Its name in the locale. */
    label: schema.string(),
    /** Its search keywords in the locale. */
    tags: schema.array(schema.string()).exactOptional(),
    /** Its group, absent for the regional indicators. */
    group: schema.number().exactOptional(),
    /** Its order within the data. */
    order: schema.number().exactOptional(),
    /** Its skin tone variants. */
    skins: schema.array(schema.looseObject({ unicode: schema.string() })).exactOptional(),
});

/** The process's own effects, which this entrypoint reads. */
const { files } = BunEnvironment.system;

/** The directory the picker's data is written to. */
const DATA = fileURLToPath(new URL("../src/emoji-picker/data/", import.meta.url));

// write each locale's emoji with the fields the picker reads, as emoji-mart ships its own data
await files.mkdir(DATA, { recursive: true });
for (const [tag, locale] of Object.entries(LOCALES)) {
    // read emojibase's compact data
    const path = fileURLToPath(import.meta.resolve(`emojibase-data/${locale}/compact.json`));
    const emoji = schema.array(CompactEmoji).parse(JSON.parse(await files.readTextFile(path)));

    // keep the six fields the picker reads
    const kept = emoji.map(({ unicode, label, tags, group, order, skins }) => ({
        unicode,
        label,
        ...(tags === undefined ? {} : { tags }),
        ...(group === undefined ? {} : { group }),
        ...(order === undefined ? {} : { order }),
        ...(skins === undefined ? {} : { skins: skins.map((skin) => ({ unicode: skin.unicode })) }),
    }));
    await files.writeFile(
        `${DATA}${tag}.json`,
        new TextEncoder().encode(`${JSON.stringify(kept)}\n`),
    );
}
