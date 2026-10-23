import { PackageId } from "@destack/package";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { schema } from "@destack/schema";
import { LocaleTag } from "../locale/locale.ts";

/** The path of a catalog in a build, capturing a dependency's package and the tag. */
const CATALOG_PATH = /^locale\/(?:([^/]+)\/)?([^/]+)\.json$/u;

/** The translations of one package's messages into one locale. */
const catalogSchema = schema.object({
    /** The package whose messages the catalog translates. */
    package: PackageId,
    /** The locale the catalog translates into. */
    locale: LocaleTag,
    /** The translated source of each message, by identifier. */
    messages: schema.record(schema.string().min(1), schema.string()),
    /** The identifiers of machine-translated messages no person reviewed yet. */
    drafts: schema.array(schema.string().min(1)).exactOptional(),
});

/** The translations of one package's messages into one locale. */
export const Catalog = Object.assign(catalogSchema, {
    /** Report whether a build path holds a catalog: the build's own, or a dependency's. */
    isPath(path: string): boolean {
        return CATALOG_PATH.test(path);
    },

    /** Write the build path of a catalog. */
    path(catalog: Pick<Catalog, "package" | "locale">, owner: PackageId): string {
        return catalog.package === owner
            ? `locale/${catalog.locale}.json`
            : `locale/${catalog.package}/${catalog.locale}.json`;
    },

    /** Read a catalog file of a build. */
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
            throw new TypeError(`catalog ${path} is for ${catalog.package}, not ${expected}`);
        } else if (catalog.locale !== tag) {
            throw new TypeError(`catalog ${path} is for ${catalog.locale}, not ${tag}`);
        }

        return catalog;
    },

    /** Read the catalogs a build ships: its own and those of its unpublished dependencies. */
    async read(reader: BuildReader): Promise<Catalog[]> {
        const owner = PackageManifest.package(reader.manifest).id;
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
