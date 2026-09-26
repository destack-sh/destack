import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import type { ESTree, Rule } from "@oxlint/plugins";
import { packageConstructors } from "@destack/package/transform";
import { isPackageFile } from "./package.ts";
import { isTestFile } from "./word.ts";

/** Package names found for linted directories, absent above the filesystem root. */
const PACKAGE_NAMES = new Map<string, string | undefined>();

/** A bare import specifier's package name, scoped or not. */
const PACKAGE_NAME = /^(?:@[^/]+\/)?[^/]+/;

/** Require declarations to be exported module constants that inspection can find. */
export const validDeclaration: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "declare as an exported module constant" },
        messages: { export: "[PK05] assign '{{name}}(...)' to an exported module-level const" },
    },
    create(context) {
        // skip test modules and files outside Destack packages
        if (isTestFile(context.filename) || !isPackageFile(context.filename)) {
            return {};
        }
        const imports = new Map<string, { name: string | undefined; source: string }>();

        return {
            ImportDeclaration(node) {
                // record each binding with its imported name, or none for namespaces
                for (const specifier of node.specifiers) {
                    const name =
                        specifier.type === "ImportSpecifier"
                            ? specifierName(specifier.imported)
                            : undefined;
                    imports.set(specifier.local.name, { name, source: node.source.value });
                }
            },
            CallExpression(node) {
                // resolve the callee to an imported constructor the build inspects
                const name = constructorName(node.callee, imports, context.filename);
                if (name === undefined) {
                    return;
                }

                // require export const name = constructor(...) at module level
                if (!isExportedConstant(node)) {
                    context.report({ node, messageId: "export", data: { name } });
                }
            },
        };
    },
};

/** Name the inspected declaration constructor a callee imports, if any. */
function constructorName(
    callee: ESTree.Node,
    imports: ReadonlyMap<string, { name: string | undefined; source: string }>,
    filename: string,
): string | undefined {
    // read named imports and namespace members
    const binding =
        callee.type === "Identifier"
            ? imports.get(callee.name)
            : callee.type === "MemberExpression" &&
                callee.object.type === "Identifier" &&
                callee.property.type === "Identifier" &&
                imports.get(callee.object.name)?.name === undefined
              ? { name: callee.property.name, source: imports.get(callee.object.name)?.source }
              : undefined;
    if (binding?.name === undefined || binding.source === undefined) {
        return undefined;
    }

    // resolve relative imports to the linted package and bare ones by package name
    const directory = dirname(filename);
    const owner = binding.source.startsWith(".")
        ? packageName(directory)
        : PACKAGE_NAME.exec(binding.source)?.[0];
    const declared = owner === undefined ? undefined : packageConstructors(owner, directory);

    return declared?.[binding.name]?.inspect === undefined ? undefined : binding.name;
}

/** Read the name of the package containing a directory, caching each directory's answer. */
function packageName(directory: string): string | undefined {
    // reuse earlier answers for shared directories
    if (PACKAGE_NAMES.has(directory)) {
        return PACKAGE_NAMES.get(directory);
    }

    // read the nearest package manifest, else ask the parent up to the filesystem root
    const manifest = join(directory, "package.json");
    const parent = dirname(directory);
    const name = existsSync(manifest)
        ? (JSON.parse(readFileSync(manifest, "utf8")).name as string)
        : parent === directory
          ? undefined
          : packageName(parent);
    PACKAGE_NAMES.set(directory, name);

    return name;
}

/** Read an import specifier's imported name. */
function specifierName(imported: ESTree.Node): string | undefined {
    return imported.type === "Identifier"
        ? imported.name
        : imported.type === "Literal" && typeof imported.value === "string"
          ? imported.value
          : undefined;
}

/** Report whether a call initializes an exported module-level const. */
function isExportedConstant(node: ESTree.Node): boolean {
    const declarator = node.parent;
    const declaration = declarator?.parent;

    return (
        declarator?.type === "VariableDeclarator" &&
        declarator.init === node &&
        declaration?.type === "VariableDeclaration" &&
        declaration.kind === "const" &&
        declaration.parent?.type === "ExportNamedDeclaration" &&
        declaration.parent.parent?.type === "Program"
    );
}
