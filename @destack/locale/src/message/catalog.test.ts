import { PackageId } from "@destack/package";
import { expect, test } from "@destack/test";
import { Catalog } from "./catalog.ts";

/** The package the catalogs translate. */
const NOTES = "package-01996ab0-0000-7000-8000-000000000001";

/** Another package, whose catalogs translate only its own messages. */
const OTHER = "package-01996ab0-0000-7000-8000-000000000002";

test("refuse a catalog whose locale is not a canonical tag", () => {
    expect([
        Catalog.safeParse({ package: NOTES, locale: "de", messages: {} }).success,
        Catalog.safeParse({ package: NOTES, locale: "de_at", messages: {} }).success,
    ]).toEqual([true, false]);
});

test("refuse a catalog that names no package", () => {
    expect(Catalog.safeParse({ locale: "de", messages: {} }).success).toBe(false);
});

test("read a build's catalog at the path naming its package and locale, refusing another package's or another locale's", () => {
    const german = { package: NOTES, locale: "de", messages: { a1: "Notizen" } };
    const read = (path: string, owner: string) => {
        try {
            return Catalog.of(path, german, PackageId.parse(owner)).locale;
        } catch (error) {
            return error instanceof TypeError ? error.message : error;
        }
    };

    expect([
        read("locale/de.json", NOTES),
        read("locale/de-AT.json", NOTES),
        read("locale/de.json", OTHER),
        read("messages/de.json", NOTES),
        read(`locale/${NOTES}/de.json`, OTHER),
        read(`locale/${OTHER}/de.json`, NOTES),
    ]).toEqual([
        "de",
        "catalog locale/de-AT.json translates de, not de-AT",
        `catalog locale/de.json translates ${NOTES}, not ${OTHER}`,
        "not a catalog path: messages/de.json",
        "de",
        `catalog locale/${OTHER}/de.json translates ${NOTES}, not ${OTHER}`,
    ]);
});

test("write a catalog's build path, a dependency's below its package", () => {
    const german = { package: PackageId.parse(NOTES), locale: "de" };
    expect([
        Catalog.path(german, PackageId.parse(NOTES)),
        Catalog.path(german, PackageId.parse(OTHER)),
    ]).toEqual(["locale/de.json", `locale/${NOTES}/de.json`]);
});
