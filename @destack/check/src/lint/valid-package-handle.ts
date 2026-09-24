import { isPackageFile } from "./package.ts";
import type { Rule } from "@oxlint/plugins";

/** Require one package handle, default-exported from src/package.ts. */
export const validPackageHandle: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "default-export the package handle from src/package.ts" },
        messages: {
            location: "[WT12] declare the package handle in src/package.ts",
            missing: "[WT12] export default definePackage(...) from src/package.ts",
        },
    },
    create(context) {
        // apply only to Destack packages
        if (!isPackageFile(context.filename)) {
            return {};
        }

        const isHandleModule = /[\\/]src[\\/]package\.ts$/.test(context.filename);

        return {
            CallExpression(node) {
                // reject handles outside their module or outside its default export
                if (node.callee.type !== "Identifier" || node.callee.name !== "definePackage") {
                    return;
                }
                if (!isHandleModule || node.parent?.type !== "ExportDefaultDeclaration") {
                    context.report({ node, messageId: "location" });
                }
            },
            Program(node) {
                // require the handle module to default-export a handle
                const handle = node.body.find(
                    (statement) =>
                        statement.type === "ExportDefaultDeclaration" &&
                        statement.declaration.type === "CallExpression" &&
                        statement.declaration.callee.type === "Identifier" &&
                        statement.declaration.callee.name === "definePackage",
                );
                if (isHandleModule && !handle) {
                    context.report({ node, messageId: "missing" });
                }
            },
        };
    },
};
