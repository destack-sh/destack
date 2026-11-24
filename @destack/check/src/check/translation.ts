import type { FileSystem } from "@destack/environment";
import { Catalog, Message } from "@destack/locale";
import { join, relative, resolve, sep } from "node:path";
import type { Diagnostic } from "../diagnostic/index.ts";
import { CONTEXT_END } from "../lint/no-missing-translations.ts";
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
    /** Its context, empty without one. */
    readonly context: string;
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

            // split its context from its source
            const end = diagnostic.message.indexOf(CONTEXT_END);
            const context = diagnostic.message.slice(0, end);
            const source = diagnostic.message.slice(end + 1);

            return { diagnostic, owner, source, context, id: Message.identify(source, context) };
        });
    const owners = Map.groupBy(messages, ({ owner }) => owner);

    // compare the messages of each package with its catalogs, leaving a test's own packages alone
    const findings: Diagnostic[] = [];
    const directories = settings.packages.map((each) => each.directory);
    for (const owner of directories.filter((path) => !isTestFile(`/${path}/`))) {
        const own = owners.get(owner) ?? [];
        const isWhole = isSelected(join(root, owner, SOURCE_DIRECTORY), options);
        if (own.length > 0 || isWhole) {
            // require each message in each language
            const catalogs = await readCatalogs(join(root, owner), files);
            findings.push(...own.flatMap((phrase) => missing(phrase, catalogs)));

            // look for translations of no message once every source was checked
            if (isWhole) {
                const used = new Set(own.map(({ id }) => id));
                findings.push(...unused(used, catalogs, directory));
            }
        }
    }

    return [...others, ...findings];
}

/** Refuse a message a language lacks, or a catalog translates with other variables. */
function missing(
    { diagnostic, id, source, context }: Phrase,
    catalogs: ReadonlyMap<string, Catalog>,
): Diagnostic[] {
    // find the languages without it, and the catalogs that translate it with other variables
    const languages = [...catalogs.values()]
        .filter((catalog) => isLanguage(catalog) && catalog.messages[id] === undefined)
        .map(({ locale }) => locale);
    const variables = variablesOf(source);
    const others = [...catalogs.values()]
        .filter(({ messages }) => variablesOf(messages[id] ?? source) !== variables)
        .map(({ locale }) => locale);
    const place = context === "" ? "" : ` in '${context}'`;
    const phrase = `'${oneLine(source)}'${place} ('${id}')`;

    return [
        ...(languages.length === 0 ? [] : [`translate ${phrase} into ${languages.join(", ")}`]),
        ...(others.length === 0
            ? []
            : [`translate ${phrase} with '${variables}' in ${others.join(", ")}`]),
    ].map((message) => ({ ...diagnostic, message }));
}

/** Refuse the translations of no message. */
function unused(
    used: ReadonlySet<string>,
    catalogs: ReadonlyMap<string, Catalog>,
    directory: string,
): Diagnostic[] {
    return [...catalogs].flatMap(([path, catalog]) =>
        Object.entries(catalog.messages)
            .filter(([id]) => !used.has(id))
            .map(([id, text]): Diagnostic => ({
                code: UNUSED,
                message: `'${id}' ('${oneLine(text)}') translates no message: remove it`,
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
function isLanguage({ locale }: Catalog): boolean {
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
): Promise<ReadonlyMap<string, Catalog>> {
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
        paths.map(async (path): Promise<[string, Catalog]> => [
            path,
            Catalog.parse(JSON.parse(await files.readTextFile(path))),
        ]),
    );

    return new Map(pairs);
}
