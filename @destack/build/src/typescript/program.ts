import { readFile } from "node:fs/promises";
import { dirname } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { Digest } from "@destack/schema";
import { type Project, type Symbol as TypeScriptSymbol } from "typescript/unstable/async";
import {
    isNoSubstitutionTemplateLiteral,
    isStringLiteral,
    type SourceFile,
    SyntaxKind,
} from "typescript/unstable/ast";
import { BuildError } from "../error/index.ts";
import { modulePackage } from "../source/dependency.ts";
import { collectDirectories } from "./directory.ts";
import { isAuthored } from "./module.ts";

/** What one import of a module resolves to. */
export type ImportTarget =
    /** One of the package's modules, by absolute path. */
    | { readonly kind: "module"; readonly file: string }
    /** A file the bundler reads and the compiler does not, such as a stylesheet, by absolute path. */
    | { readonly kind: "file"; readonly file: string }
    /** A dependency, compiler library, runtime builtin or virtual module, named by its specifier alone. */
    | { readonly kind: "external" };

/** One of the package's modules: its bytes, its imports and the directories it reads. */
export interface ModuleImports {
    /** The SHA-256 digest of the module's bytes. */
    readonly digest: Digest;
    /** The module's imports, in source order. */
    readonly imports: readonly { readonly specifier: string; readonly target: ImportTarget }[];
    /** The absolute directories the module reads through literal directory URLs. */
    readonly directories: readonly string[];
}

/** A compiler program as a cache key sees it: its options, its packages and the package's modules. */
export interface ProgramImports {
    /** The effective compiler options as JSON, without the location of the generated configuration. */
    readonly options: string;
    /** The package directory of every program file outside the package, compiler libraries included. */
    readonly packages: readonly string[];
    /** The package's modules, by absolute path. */
    readonly modules: ReadonlyMap<string, ModuleImports>;
}

/** Read a program's options, packages and the imports of the package's modules, without checking them. */
export async function collectImports(project: Project, root: string): Promise<ProgramImports> {
    // read the package's modules
    const fileNames = await project.program.getSourceFileNames();
    const sources = await Promise.all(
        fileNames
            .filter((file) => isAuthored(root, file))
            .map(async (file) => {
                const source = await project.program.getSourceFile(file);
                if (!source) {
                    throw new BuildError("INSPECTION_FAILED", `missing compiler source: ${file}`);
                }

                return source;
            }),
    );
    const authored = new Map(sources.map((source) => [source.path, source.fileName]));

    // locate the package of every other program file once per directory
    const directories = new Map<string, Promise<string>>();
    const packages = await Promise.all(
        fileNames
            .filter((file) => !isAuthored(root, file))
            .map((file) => {
                let location = directories.get(dirname(file));
                if (location === undefined) {
                    location = modulePackage(dirname(file)).then((owner) => owner.directory);
                    directories.set(dirname(file), location);
                }

                return location;
            }),
    );

    // read each module's imports
    const modules = await Promise.all(
        sources.map(
            async (source) =>
                [source.fileName, await readImports(source, project, authored)] as const,
        ),
    );
    const options = Object.entries(project.program.getCompilerOptions()).filter(
        ([key]) => key !== "configFilePath",
    );

    return {
        options: JSON.stringify(Object.fromEntries(options)),
        packages: [...new Set(packages)].toSorted(),
        modules: new Map(modules),
    };
}

/** Read a module's digest, resolved imports and directory references. */
async function readImports(
    source: SourceFile,
    project: Project,
    authored: ReadonlyMap<string, string>,
): Promise<ModuleImports> {
    // read literal specifiers, including dynamic imports
    const specifiers = source.imports.map((entry) => {
        if (!isStringLiteral(entry) && !isNoSubstitutionTemplateLiteral(entry)) {
            throw new BuildError("INSPECTION_FAILED", `unsupported import in ${source.fileName}`);
        }

        return entry;
    });

    // resolve the specifiers and read the module's bytes and directory references together
    const [symbols, bytes, references] = await Promise.all([
        specifiers.length ? project.checker.getSymbolAtLocation(specifiers) : [],
        readFile(source.fileName),
        collectDirectories(source, project),
    ]);
    const imports = specifiers.map((entry, index) => ({
        specifier: entry.text,
        target: importTarget(source, entry.text, symbols[index], authored),
    }));

    return {
        digest: await Digest.of(new Uint8Array(bytes)),
        imports,
        directories: references.map((reference) =>
            fileURLToPath(new URL(reference.path, pathToFileURL(source.fileName))),
        ),
    };
}

/** Resolve what an import names: one of the package's modules, a file the bundler reads, or something external. */
function importTarget(
    source: SourceFile,
    specifier: string,
    symbol: TypeScriptSymbol | undefined,
    authored: ReadonlyMap<string, string>,
): ImportTarget {
    // locate the module the compiler resolves
    const declaration = symbol?.declarations[0];
    const isModule = declaration?.kind === SyntaxKind.SourceFile;
    const file = isModule ? authored.get(declaration.path) : undefined;

    // name one of the package's modules by its path
    if (file !== undefined) {
        return { kind: "module", file };
    }
    // name a relative file the compiler does not resolve by its path
    else if (!isModule && /^\.\.?\//u.test(specifier)) {
        return {
            kind: "file",
            file: fileURLToPath(new URL(specifier, pathToFileURL(source.fileName))),
        };
    }
    // name anything else by its specifier
    else {
        return { kind: "external" };
    }
}
