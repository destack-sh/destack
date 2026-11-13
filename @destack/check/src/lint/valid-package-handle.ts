import { isPackageFile, packageOf } from "./package.ts";
import { isTestFile } from "./word.ts";
import type { Rule } from "@oxlint/plugins";

/** Require one package handle, default-exported from src/package.ts, described as its package.json. */
export const validPackageHandle: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "default-export the package handle from src/package.ts" },
        messages: {
            location: "declare the package handle in src/package.ts",
            missing: "export default definePackage(...) from src/package.ts",
            description:
                "set package.json's description to the handle's, '{{ handle }}', or write the handle's as plain t`...` text",
        },
    },
    create(context) {
        // apply only to Destack packages, leaving tests their handles
        if (isTestFile(context.filename) || !isPackageFile(context.filename, context.settings)) {
            return {};
        }

        const isHandleModule = /[\\/]src[\\/]package\.ts$/u.test(context.filename);

        return {
            CallExpression(node) {
                // reject handles outside their module or outside its default export
                if (node.callee.type !== "Identifier" || node.callee.name !== "definePackage") {
                    return;
                }
                if (!isHandleModule || node.parent?.type !== "ExportDefaultDeclaration") {
                    context.report({ node, messageId: "location" });
                    return;
                }

                // require package.json's description to read as the handle's in the source locale
                const [options] = node.arguments;
                const property =
                    options?.type === "ObjectExpression"
                        ? options.properties.find(
                              (each) =>
                                  each.type === "Property" &&
                                  each.key.type === "Identifier" &&
                                  each.key.name === "description",
                          )
                        : undefined;
                const value = property?.type === "Property" ? property.value : undefined;
                const handle =
                    value?.type === "TaggedTemplateExpression" &&
                    value.tag.type === "Identifier" &&
                    value.tag.name === "t" &&
                    value.quasi.expressions.length === 0
                        ? value.quasi.quasis[0]?.value.cooked
                        : undefined;
                const manifest = packageOf(context.filename, context.settings)?.description;
                if (value !== undefined && (handle === undefined || handle !== manifest)) {
                    context.report({
                        node: value,
                        messageId: "description",
                        data: { handle: handle ?? "" },
                    });
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
