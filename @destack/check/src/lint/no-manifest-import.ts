import { isPackageFile } from "./package.ts";
import type { Rule } from "@oxlint/plugins";
import { isTestFile } from "./word.ts";

/** Reject reading package manifests from package source. */
export const noManifestImport: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "read package identity from import.meta.destack or a package handle" },
        messages: {
            manifest:
                "[PK07] read package identity from import.meta.destack or a package handle instead of {{file}}",
        },
    },
    create(context) {
        // apply only to Destack packages
        if (!isPackageFile(context.filename)) {
            return {};
        }

        return {
            ImportDeclaration(node) {
                // match manifest files imported from source modules
                const file = node.source.value.split("/").at(-1);
                if (
                    /[\\/]src[\\/]/.test(context.filename) &&
                    !isTestFile(context.filename) &&
                    (file === "destack.json" || file === "package.json")
                ) {
                    context.report({ node, messageId: "manifest", data: { file } });
                }
            },
        };
    },
};
