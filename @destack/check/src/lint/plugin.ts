import type { Rule } from "@oxlint/plugins";
import { booleanPrefix } from "./boolean-prefix.ts";
import { branchCommentPosition } from "./branch-comment-position.ts";
import { commentStyle } from "./comment-style.ts";
import { errorMessageStyle } from "./error-message-style.ts";
import { jsdocSentence } from "./jsdoc-sentence.ts";
import { noImportAlias } from "./no-import-alias.ts";
import { noIndexLogic } from "./no-index-logic.ts";
import { noInlineConfiguration } from "./no-inline-config.ts";
import { noManifestImport } from "./no-manifest-import.ts";
import { noPartialAssertions } from "./no-partial-assertions.ts";
import { paddingBeforeReturn } from "./padding-before-return.ts";
import { preventAbbreviations } from "./prevent-abbreviations.ts";
import { requireBlockComment } from "./require-block-comment.ts";
import { requireJsdoc } from "./require-jsdoc.ts";
import { validDeclaration } from "./valid-declaration.ts";
import { validPackageHandle } from "./valid-package-handle.ts";

/** A trusted Oxc plugin selected by the host's dependency resolver. */
export interface Plugin {
    /** Unique rule namespace. */
    name: string;
    /** Absolute module path for managed checks, or package export for editor configuration. */
    specifier: string;
    /** Rules enabled as errors whenever this plugin is selected. */
    rules: Record<string, Rule>;
}

/** Mandatory Destack source rules. */
export const rules: Record<string, Rule> = {
    "boolean-prefix": booleanPrefix,
    "branch-comment-position": branchCommentPosition,
    "comment-style": commentStyle,
    "error-message-style": errorMessageStyle,
    "jsdoc-sentence": jsdocSentence,
    "no-import-alias": noImportAlias,
    "no-index-logic": noIndexLogic,
    "no-inline-config": noInlineConfiguration,
    "no-manifest-import": noManifestImport,
    "no-partial-assertions": noPartialAssertions,
    "padding-before-return": paddingBeforeReturn,
    "prevent-abbreviations": preventAbbreviations,
    "require-block-comment": requireBlockComment,
    "require-jsdoc": requireJsdoc,
    "valid-declaration": validDeclaration,
    "valid-package-handle": validPackageHandle,
};

/** Mandatory Destack source rules. */
const plugin = {
    meta: { name: "destack" },
    rules,
};

export default plugin;
