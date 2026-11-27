import { ModuleResolution } from "@destack/package/manifest";
import { Digest } from "@destack/schema";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { ImageBundle } from "./image.ts";

/** The prefixes of the builds' files and modules. */
const PREFIXES = { builds: "/.destack/build/", modules: "/.destack/module/" };

/** The image's root build. */
const ROOT = Digest.parse("a".repeat(64));
/** A package whose modules run code on import. */
const EFFECTFUL = Digest.parse("b".repeat(64));
/** A package declaring `"sideEffects": false`. */
const PURE = Digest.parse("c".repeat(64));
/** A package declaring `"sideEffects": false`, which the bundle leaves external. */
const SHARED = Digest.parse("d".repeat(64));

/** Each build's one module, logging its name on import and exporting a value. */
const MODULES: ReadonlyMap<Digest, string> = new Map([
    [
        ROOT,
        'import "effectful";\nimport { unused } from "pure";\nimport { other } from "shared";\nexport const root = 1;\n',
    ],
    [EFFECTFUL, 'console.log("effectful");\nexport const effect = 1;\n'],
    [PURE, 'console.log("pure");\nexport const unused = 1;\n'],
    [SHARED, 'console.log("shared");\nexport const other = 1;\n'],
]);

test("drop unused imports of modules without side effects, bundled or external", async () => {
    // resolve an image whose root imports three packages, using none of their exports
    const crypto = testHost().crypto;
    const at = (build: Digest) => `${PREFIXES.builds}${build}/index.js`;
    const resolution = await ModuleResolution.of(
        {
            imports: { effectful: at(EFFECTFUL), pure: at(PURE), shared: at(SHARED) },
            scopes: Object.fromEntries(
                [EFFECTFUL, PURE, SHARED].map((build) => [`${PREFIXES.builds}${build}/`, {}]),
            ),
        },
        ROOT,
        PREFIXES,
        async () => [],
        crypto,
    );
    const buildOf = (url: string) => resolution.locate(url)?.build;

    // bundle the root, the shared package left external, the pure ones declared side effect free
    const bundle = await ImageBundle.of(resolution, [resolution.url(ROOT, "index.js")], {
        directory: "output/browser",
        load: async (url) => MODULES.get(buildOf(url) ?? ROOT),
        map: async () => undefined,
        asset: async () => undefined,
        hasSideEffects: async (url) => buildOf(url) === EFFECTFUL,
        external: (url) => (buildOf(url) === SHARED ? "shared" : undefined),
        linked: new Set(),
    });
    const logged = bundle.chunks.flatMap((chunk) =>
        [...chunk.code.matchAll(/console\.log\(["'`](\w+)["'`]\)/gu)].map((match) => match[1]),
    );

    // expect only the effectful module's code, and no import of the unused shared package
    expect({
        logged,
        imports: bundle.chunks.flatMap((chunk) => chunk.imports),
    }).toEqual({ logged: ["effectful"], imports: [] });
});
