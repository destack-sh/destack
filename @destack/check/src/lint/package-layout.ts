import { PackageDefinition } from "@destack/package";
import type { ESTree, Rule } from "@oxlint/plugins";
import { LintSettings, packageOf } from "./package.ts";
import { isTestFile, isTestKitFile } from "./word.ts";

/** A bare specifier's package name and subpath: `@scope/name` or `name`, then the rest. */
const SPECIFIER = /^((?:@[^/]+\/)?[^/]+)(\/.*)?$/u;

/** Refuse imports the package layout forbids, as the build does. */
export const packageLayout: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "import only the subpaths the package layout allows" },
        messages: { layout: "{{message}}" },
    },
    create(context) {
        // apply only to files of the listed packages
        const name = packageOf(context.filename, context.settings)?.name;
        if (name === undefined) {
            return {};
        }
        const { roles } = LintSettings.read(context.settings);
        const role = roles[name];
        const isTest = isTestFile(context.filename) || isTestKitFile(context.filename);

        // check a value import or re-export of another Destack package against its role
        const check = (
            node:
                | ESTree.ImportDeclaration
                | ESTree.ExportNamedDeclaration
                | ESTree.ExportAllDeclaration,
        ) => {
            // read the imported package and its role, leaving type imports and other packages
            const kind = node.type === "ImportDeclaration" ? node.importKind : node.exportKind;
            const matched = node.source === null ? null : SPECIFIER.exec(node.source.value);
            const imported = matched?.[1];
            if (imported === undefined || role === undefined || kind === "type") {
                return;
            }
            const importedRole = roles[imported];
            if (importedRole === undefined) {
                return;
            }
            try {
                PackageDefinition.requireImport(
                    { name, role, isTest },
                    { name: imported, role: importedRole },
                    `.${matched?.[2] ?? ""}`,
                );
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                context.report({ node, messageId: "layout", data: { message } });
            }
        };

        return {
            ImportDeclaration: check,
            ExportNamedDeclaration: check,
            ExportAllDeclaration: check,
        };
    },
};
