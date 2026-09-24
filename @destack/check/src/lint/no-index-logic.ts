import { isPackageFile } from "./package.ts";
import type { Rule } from "@oxlint/plugins";

/** Restrict index modules to re-exports. */
export const noIndexLogic: Rule = {
    meta: {
        type: "suggestion",
        schema: [],
        docs: { description: "keep index modules to re-exports" },
        messages: { logic: "[WT14] move this statement out of the index module" },
    },
    create(context) {
        // apply only to Destack packages
        if (!isPackageFile(context.filename)) {
            return {};
        }

        return {
            Program(node) {
                // allow only export-from statements in index modules
                if (!/[\\/]index\.[cm]?[jt]sx?$/.test(context.filename)) {
                    return;
                }
                for (const statement of node.body) {
                    const isReexport =
                        statement.type === "ExportAllDeclaration" ||
                        (statement.type === "ExportNamedDeclaration" && statement.source !== null);
                    if (!isReexport) {
                        context.report({ node: statement, messageId: "logic" });
                    }
                }
            },
        };
    },
};
