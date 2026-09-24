import type { Rule } from "@oxlint/plugins";

/** Global names that imports may rename to avoid shadowing. */
const GLOBALS = new Set([
    "Symbol",
    "Map",
    "Set",
    "Node",
    "Event",
    "Error",
    "Request",
    "Response",
    "Headers",
    "URL",
    "File",
    "Blob",
    "Document",
    "Element",
    "Text",
    "Comment",
    "Promise",
    "Proxy",
    "Object",
    "Array",
    "String",
    "Number",
    "Boolean",
    "Function",
    "Date",
    "RegExp",
    "Iterator",
    "Worker",
    "Location",
    "Storage",
    "Range",
    "Selection",
    "Window",
    "Plugin",
    "Module",
]);

/** Reject renamed imports so each name reads the same everywhere. */
export const noImportAlias: Rule = {
    meta: {
        type: "suggestion",
        schema: [],
        docs: { description: "import names without renaming them" },
        messages: { alias: "[WR14] import '{{name}}' under its own name" },
    },
    create(context) {
        return {
            ImportSpecifier(node) {
                // compare the exported and local names
                const imported =
                    node.imported.type === "Identifier" ? node.imported.name : undefined;
                if (imported && imported !== node.local.name && !GLOBALS.has(imported)) {
                    context.report({ node, messageId: "alias", data: { name: imported } });
                }
            },
        };
    },
};
