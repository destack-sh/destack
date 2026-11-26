import type { FileSystem } from "@destack/environment";
import { CatalogSource, Message, MESSAGE_CONTEXTS, type MessageContext } from "@destack/locale";
import { join, relative, resolve, sep } from "node:path";
import type { Diagnostic } from "../diagnostic/index.ts";
import { MESSAGE_SEPARATOR } from "../lint/no-missing-translations.ts";
import { type LintSettings, packageOf } from "../lint/package.ts";
import { isTestFile } from "../lint/word.ts";

/** The directory of a package's catalogs. */
const CATALOG_DIRECTORY = "locale";

/** The directory of a package's sources. */
const SOURCE_DIRECTORY = "src";

/** A catalog file. */
const CATALOG_FILE = /\.json$/u;

/** A variable of a message. */
const VARIABLE = /\$[a-zA-Z0-9]+/gu;

/** The code of a message without a translation, which the rule reports each message under. */
const MISSING = "destack(no-missing-translations)";

/** The code of a translation of no message. */
const UNUSED = "destack(no-unused-translations)";

/** The code of a translation whose source, context or note differs from its message's. */
const STALE = "destack(no-stale-translations)";

/** The files of a check whose messages the catalogs translate. */
export interface TranslationOptions {
    /** The directory the findings' files are relative to. */
    readonly directory: string;
    /** The directory the settings' packages are relative to. */
    readonly root: string;
    /** The packages. */
    readonly settings: LintSettings;
    /** The checked files and directories. */
    readonly selection: readonly string[];
    /** The files with the catalogs. */
    readonly files: FileSystem;
    /** Whether to write the catalogs from the code, dropping the translations of no message. */
    readonly isFix: boolean;
}

/** A message a check found in a package's sources. */
interface Phrase {
    /** The finding at it. */
    readonly diagnostic: Diagnostic;
    /** The directory of its package. */
    readonly owner: string;
    /** The identifier catalogs translate it under. */
    readonly id: string;
    /** Its source text. */
    readonly source: string;
    /** Its context, absent for a sentence or a phrase. */
    readonly context: MessageContext | undefined;
    /** The note its writer leaves its translators, absent without one. */
    readonly note: string | undefined;
}

/** Keep the messages a catalog lacks, and add the translations of no message. */
export async function applyTranslations(
    diagnostics: readonly Diagnostic[],
    options: TranslationOptions,
): Promise<Diagnostic[]> {
    // group the messages by their package's directory
    const { directory, root, settings, files } = options;
    const others = diagnostics.filter((diagnostic) => diagnostic.code !== MISSING);
    const messages = diagnostics
        .filter((diagnostic) => diagnostic.code === MISSING)
        .map((diagnostic): Phrase => {
            // find the package the message is of
            const file = resolve(directory, diagnostic.filename);
            const owner = packageOf(file, { destack: settings })?.directory ?? ".";

            // split its context and note from its source
            const [context = "", note = "", ...rest] = diagnostic.message.split(MESSAGE_SEPARATOR);
            const source = rest.join(MESSAGE_SEPARATOR);
            const id = Message.identify(source, context);

            return {
                diagnostic,
                owner,
                source,
                context: MESSAGE_CONTEXTS.find((each) => each === context),
                note: note === "" ? undefined : note,
                id,
            };
        });
    const owners = Map.groupBy(messages, ({ owner }) => owner);

    // compare the messages of each package with its catalogs, leaving a test's own packages alone
    const findings: Diagnostic[] = [];
    const directories = settings.packages.map((each) => each.directory);
    for (const owner of directories.filter((path) => !isTestFile(`/${path}/`))) {
        const own = owners.get(owner) ?? [];
        const isWhole = isSelected(join(root, owner, SOURCE_DIRECTORY), options);
        if (own.length > 0 || isWhole) {
            // require each message in each language, with the code's source, context and note
            const catalogs = await readCatalogs(join(root, owner), files);
            findings.push(...own.flatMap((phrase) => missing(phrase, catalogs)));

            // rewrite each catalog from the code, else refuse what differs from it
            const used = isWhole ? new Set(own.map(({ id }) => id)) : undefined;
            if (options.isFix) {
                await rewrite(own, used, catalogs, files);
            } else {
                findings.push(...own.flatMap((phrase) => stale(phrase, catalogs, directory)));
                findings.push(...(used === undefined ? [] : unused(used, catalogs, directory)));
            }
        }
    }

    return [...others, ...findings];
}

/** Refuse a message a language lacks, or a catalog translates with other variables. */
function missing(
    { diagnostic, id, source, context }: Phrase,
    catalogs: ReadonlyMap<string, CatalogSource>,
): Diagnostic[] {
    // find the languages without it, and the catalogs that translate it with other variables
    const languages = [...catalogs.values()]
        .filter((catalog) => isLanguage(catalog) && catalog.messages[id] === undefined)
        .map(({ locale }) => locale);
    const variables = variablesOf(source);
    const others = [...catalogs.values()]
        .filter(({ messages }) => variablesOf(messages[id]?.translation ?? source) !== variables)
        .map(({ locale }) => locale);
    const place = context === undefined ? "" : ` in '${context}'`;
    const phrase = `'${oneLine(source)}'${place} ('${id}')`;

    return [
        ...(languages.length === 0 ? [] : [`translate ${phrase} into ${languages.join(", ")}`]),
        ...(others.length === 0
            ? []
            : [`translate ${phrase} with '${variables}' in ${others.join(", ")}`]),
    ].map((message) => ({ ...diagnostic, message }));
}

/** Refuse a translation whose source, context or note differs from its message's. */
function stale(
    phrase: Phrase,
    catalogs: ReadonlyMap<string, CatalogSource>,
    directory: string,
): Diagnostic[] {
    return [...catalogs].flatMap(([path, catalog]): Diagnostic[] => {
        // take a translation that differs from the message
        const translation = catalog.messages[phrase.id];
        if (translation === undefined || isCurrent(translation, phrase)) {
            return [];
        }

        return [
            {
                code: STALE,
                message: `'${phrase.id}' ('${oneLine(phrase.source)}') differs from its message's source, context or note: run destack fix`,
                severity: "error",
                filename: relative(directory, path),
                labels: [],
            },
        ];
    });
}

/** Write each catalog's sources, contexts and notes from the code, dropping the unused. */
async function rewrite(
    phrases: readonly Phrase[],
    used: ReadonlySet<string> | undefined,
    catalogs: ReadonlyMap<string, CatalogSource>,
    files: FileSystem,
): Promise<void> {
    for (const [path, catalog] of catalogs) {
        // take each message's source, context and note from the code, its translation as it is
        const messages = { ...catalog.messages };
        for (const phrase of phrases) {
            const translation = messages[phrase.id];
            if (translation !== undefined) {
                messages[phrase.id] = {
                    source: phrase.source,
                    ...(phrase.context === undefined ? {} : { context: phrase.context }),
                    ...(phrase.note === undefined ? {} : { note: phrase.note }),
                    translation: translation.translation,
                };
            }
        }

        // drop the translations of no message, once every source was read
        const kept = Object.entries(messages).filter(([id]) => used === undefined || used.has(id));
        const written = { ...catalog, messages: Object.fromEntries(kept) };
        if (JSON.stringify(written) !== JSON.stringify(catalog)) {
            await files.writeTextFile(path, `${JSON.stringify(written, null, 4)}\n`);
        }
    }
}

/** Report whether a translation has its message's source, context and note. */
function isCurrent(
    translation: CatalogSource["messages"][string],
    { source, context, note }: Phrase,
): boolean {
    return (
        translation.source === source &&
        translation.context === context &&
        translation.note === note
    );
}

/** Refuse the translations of no message. */
function unused(
    used: ReadonlySet<string>,
    catalogs: ReadonlyMap<string, CatalogSource>,
    directory: string,
): Diagnostic[] {
    return [...catalogs].flatMap(([path, catalog]) =>
        Object.entries(catalog.messages)
            .filter(([id]) => !used.has(id))
            .map(([id, { source }]): Diagnostic => ({
                code: UNUSED,
                message: `'${id}' ('${oneLine(source)}') translates no message: run destack fix`,
                severity: "error",
                filename: relative(directory, path),
                labels: [],
            })),
    );
}

/** List the variables of a message, in order of their names. */
function variablesOf(text: string): string {
    return [...new Set(text.match(VARIABLE))].toSorted().join(", ");
}

/** Write a message with variants on one line. */
function oneLine(text: string): string {
    return text.replaceAll("\n", " ");
}

/** Report whether a catalog is of a language, which its regions' catalogs fall back to. */
function isLanguage({ locale }: CatalogSource): boolean {
    return new Intl.Locale(locale).region === undefined;
}

/** Report whether a check's selection has a path. */
function isSelected(path: string, { directory, selection }: TranslationOptions): boolean {
    return selection
        .map((entry) => resolve(directory, entry))
        .some((entry) => path === entry || path.startsWith(`${entry}${sep}`));
}

/** Read the catalogs of a package's own messages by file, none for a package without them. */
async function readCatalogs(
    directory: string,
    files: FileSystem,
): Promise<ReadonlyMap<string, CatalogSource>> {
    // take a package without catalogs as it is
    const catalogs = join(directory, CATALOG_DIRECTORY);
    if (!(await files.exists(catalogs))) {
        return new Map();
    }

    // read each catalog, in order of the files' names
    const entries = await files.readDirectory(catalogs);
    const names = entries.map(({ name }) => name).filter((name) => CATALOG_FILE.test(name));
    const paths = names.toSorted().map((name) => join(catalogs, name));
    const pairs = await Promise.all(
        paths.map(async (path): Promise<[string, CatalogSource]> => [
            path,
            CatalogSource.parse(JSON.parse(await files.readTextFile(path))),
        ]),
    );

    return new Map(pairs);
}
