import { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { defineSchema, schema } from "@destack/schema";
import { LocaleTag } from "../locale/locale.ts";

/** The path of a catalog in a build, capturing a dependency's package and the tag: `locale/<tag>.json` for the build's own package, `locale/<package>/<tag>.json` for a dependency's. */
const CATALOG_PATH = /^locale\/(?:([^/]+)\/)?([^/]+)\.json$(?![\s\S])/u;

/** The translations of one package's messages into one locale, as MessageFormat 2 sources by message identifier. */
const catalogSchema = defineSchema(
    schema.object({
        /** The package whose messages the catalog translates. */
        package: PackageId,
        /** The locale the catalog translates into. */
        locale: LocaleTag,
        /** The translated source of each message, by identifier. */
        messages: schema.record(schema.string().min(1), schema.string()),
        /** The identifiers of machine-translated messages no person reviewed yet. */
        drafts: schema.array(schema.string().min(1)).exactOptional(),
    }),
);

/** The translations of one package's messages into one locale, kept at `locale/<tag>.json` in the package. */
export const Catalog = Object.assign(catalogSchema, {
    /** Report whether a build path holds a catalog: the build's own, or a dependency's. */
    isPath(path: string): boolean {
        return CATALOG_PATH.test(path);
    },

    /** Write the build path of a catalog: `locale/<tag>.json` for the build's own package, `locale/<package>/<tag>.json` for a dependency's. */
    path(catalog: Pick<Catalog, "package" | "locale">, owner: PackageId): string {
        return catalog.package === owner
            ? `locale/${catalog.locale}.json`
            : `locale/${catalog.package}/${catalog.locale}.json`;
    },

    /** Read a catalog file of a build, refusing one of a package or a locale its path does not name. */
    of(path: string, value: unknown, owner: PackageId): Catalog {
        // require a catalog path and a catalog
        const [, dependency, tag] = CATALOG_PATH.exec(path) ?? [];
        if (tag === undefined) {
            throw new TypeError(`not a catalog path: ${path}`);
        }
        const catalog = catalogSchema.parse(value);

        // require the package and the locale the path names
        const expected = dependency ?? owner;
        if (catalog.package !== expected) {
            throw new TypeError(`catalog ${path} translates ${catalog.package}, not ${expected}`);
        } else if (catalog.locale !== tag) {
            throw new TypeError(`catalog ${path} translates ${catalog.locale}, not ${tag}`);
        }

        return catalog;
    },

    /** Read the catalogs a build ships, its own and those of its unpublished dependencies, verifying each file. */
    async read(reader: BuildReader): Promise<Catalog[]> {
        const owner = reader.manifest.package.id;
        const files = (await reader.files()).filter((file) => Catalog.isPath(file.path));

        return Promise.all(
            files.map(async (file) =>
                Catalog.of(file.path, await reader.read(file, schema.json()), owner),
            ),
        );
    },
});

/** The translations of one package's messages into one locale. */
export type Catalog = schema.Infer<typeof catalogSchema>;
