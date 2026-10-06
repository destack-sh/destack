import type { Rule } from "@oxlint/plugins";
import { booleanPrefix } from "./boolean-prefix.ts";
import { branchCommentPosition } from "./branch-comment-position.ts";
import { commentStyle } from "./comment-style.ts";
import { errorMessageStyle } from "./error-message-style.ts";
import { exactOptional } from "./exact-optional.ts";
import { jsdocSentence } from "./jsdoc-sentence.ts";
import { noClassName } from "./no-class-name.ts";
import { noImportAlias } from "./no-import-alias.ts";
import { noIndexLogic } from "./no-index-logic.ts";
import { noInlineConfiguration } from "./no-inline-config.ts";
import { noManifestImport } from "./no-manifest-import.ts";
import { noOverloadCast } from "./no-overload-cast.ts";
import { noPartialAssertions } from "./no-partial-assertions.ts";
import { paddingBeforeReturn } from "./padding-before-return.ts";
import { preventAbbreviations } from "./prevent-abbreviations.ts";
import { requireBlockComment } from "./require-block-comment.ts";
import { requireJsdoc } from "./require-jsdoc.ts";
import { styleAttribute } from "./style-attribute.ts";
import { styleHover } from "./style-hover.ts";
import { styleShorthand } from "./style-shorthand.ts";
import { styleTokens } from "./style-tokens.ts";
import { styleXstyleLast } from "./style-xstyle-last.ts";
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
export const rules = {
    "boolean-prefix": booleanPrefix,
    "branch-comment-position": branchCommentPosition,
    "comment-style": commentStyle,
    "error-message-style": errorMessageStyle,
    "exact-optional": exactOptional,
    "jsdoc-sentence": jsdocSentence,
    "no-class-name": noClassName,
    "no-import-alias": noImportAlias,
    "no-index-logic": noIndexLogic,
    "no-inline-config": noInlineConfiguration,
    "no-manifest-import": noManifestImport,
    "no-overload-cast": noOverloadCast,
    "no-partial-assertions": noPartialAssertions,
    "padding-before-return": paddingBeforeReturn,
    "prevent-abbreviations": preventAbbreviations,
    "require-block-comment": requireBlockComment,
    "require-jsdoc": requireJsdoc,
    "style-attribute": styleAttribute,
    "style-hover": styleHover,
    "style-shorthand": styleShorthand,
    "style-tokens": styleTokens,
    "style-xstyle-last": styleXstyleLast,
    "valid-declaration": validDeclaration,
    "valid-package-handle": validPackageHandle,
} satisfies Record<string, Rule>;

/** Mandatory Destack source rules. */
const plugin = {
    meta: { name: "destack" },
    rules,
};

export default plugin;
