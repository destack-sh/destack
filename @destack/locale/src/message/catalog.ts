import { PackageId } from "@destack/package";
import { type BuildReader, CATALOG_DIRECTORY, PackageManifest } from "@destack/package/manifest";
import { schema } from "@destack/schema";
import { LocaleTag } from "../locale/locale.ts";
import { MESSAGE_CONTEXTS } from "./message.ts";

/** The path of a catalog in a build, capturing a dependency's package and the tag. */
const CATALOG_PATH = new RegExp(`^${CATALOG_DIRECTORY}/(?:([^/]+)/)?([^/]+)\\.json$`, "u");

/** The translations of one package's messages into one locale. */
const catalogSchema = schema.object({
    /** The package whose messages the catalog translates. */
    package: PackageId,
    /** The locale the catalog translates into. */
    locale: LocaleTag,
    /** The translated source of each message, by identifier. */
    messages: schema.record(schema.string().min(1), schema.string()),
});

/** The translations of one package's messages into one locale. */
export const Catalog = Object.assign(catalogSchema, {
    /** Report whether a build path holds a catalog: the build's own, or a dependency's. */
    isPath(path: string): boolean {
        return CATALOG_PATH.test(path);
    },

    /** Read a catalog path's dependency and locale, absent for a path of no catalog. */
    at(path: string): { readonly dependency?: PackageId; readonly locale: LocaleTag } | undefined {
        // read the path's parts, none for another path
        const [, dependency, tag] = CATALOG_PATH.exec(path) ?? [];
        if (tag === undefined) {
            return undefined;
        }
        const locale = LocaleTag.parse(tag);

        return dependency === undefined
            ? { locale }
            : { dependency: PackageId.parse(dependency), locale };
    },

    /** Write the build path of a catalog. */
    path(catalog: Pick<Catalog, "package" | "locale">, owner: PackageId): string {
        return catalog.package === owner
            ? `${CATALOG_DIRECTORY}/${catalog.locale}.json`
            : `${CATALOG_DIRECTORY}/${catalog.package}/${catalog.locale}.json`;
    },

    /** Read a catalog file of a build. */
    of(path: string, value: unknown, owner: PackageId): Catalog {
        // require a catalog path and a catalog
        const at = Catalog.at(path);
        if (at === undefined) {
            throw new TypeError(`not a catalog path: '${path}'`);
        }
        const catalog = catalogSchema.parse(value);

        // require the package and the locale of the path
        const expected = at.dependency ?? owner;
        if (catalog.package !== expected) {
            throw new TypeError(`catalog '${path}' is for '${catalog.package}', not '${expected}'`);
        } else if (catalog.locale !== at.locale) {
            throw new TypeError(`catalog '${path}' is for '${catalog.locale}', not '${at.locale}'`);
        }

        return catalog;
    },

    /** Read the catalogs a build ships: its own and those of its unpublished dependencies. */
    async read(reader: BuildReader): Promise<Catalog[]> {
        const owner = PackageManifest.package(reader.manifest).id;
        const files = (await reader.files()).filter((file) => Catalog.isPath(file.path));

        return Promise.all(
            files.map(async (file) => Catalog.of(file.path, await reader.read(file), owner)),
        );
    },
});

/** The translations of one package's messages into one locale. */
export type Catalog = schema.Infer<typeof catalogSchema>;

/** One message's translation as a catalog source writes it, with what its translators read. */
export const Translation = schema.object({
    /** The message's source text, as MessageFormat 2. */
    source: schema.string(),
    /** What the message is where it shows, absent for a sentence or a phrase. */
    context: schema.enum(MESSAGE_CONTEXTS).exactOptional(),
    /** The note its writer leaves its translators, absent without one. */
    note: schema.string().min(1).exactOptional(),
    /** The translated source, as MessageFormat 2. */
    translation: schema.string(),
});
/** One message's translation as a catalog source writes it. */
export type Translation = schema.Infer<typeof Translation>;

/** A catalog as its package writes it, after XLIFF 2 and String Catalogs. */
const catalogSourceSchema = schema.object({
    /** The package whose messages the catalog translates. */
    package: PackageId,
    /** The locale the catalog translates into. */
    locale: LocaleTag,
    /** Each message's translation, by identifier. */
    messages: schema.record(schema.string().min(1), Translation),
});

/** A catalog as its package writes it, which a build compiles to the catalog it ships. */
export const CatalogSource = Object.assign(catalogSourceSchema, {
    /** Compile a catalog source to the catalog a build ships: each message's translation. */
    compile(source: CatalogSource): Catalog {
        // take each message's translation by its identifier
        const messages = Object.fromEntries(
            Object.entries(source.messages).map(([id, { translation }]) => [id, translation]),
        );

        return { package: source.package, locale: source.locale, messages };
    },
});
/** A catalog as its package writes it. */
export type CatalogSource = schema.Infer<typeof catalogSourceSchema>;
