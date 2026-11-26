import type { Rule } from "@oxlint/plugins";
import { booleanPrefix } from "./boolean-prefix.ts";
import { branchCommentPosition } from "./branch-comment-position.ts";
import { commentStyle } from "./comment-style.ts";
import { deterministicHandler } from "./deterministic-handler.ts";
import { errorMessageStyle } from "./error-message-style.ts";
import { exactOptional } from "./exact-optional.ts";
import { jsdocSentence } from "./jsdoc-sentence.ts";
import { methodSignatureStyle } from "./method-signature-style.ts";
import { noAmbientEffect } from "./no-ambient-effect.ts";
import { noClassName } from "./no-class-name.ts";
import { noImportAlias } from "./no-import-alias.ts";
import { noIndexLogic } from "./no-index-logic.ts";
import { noInlineConfiguration } from "./no-inline-config.ts";
import { messageContext } from "./message-context.ts";
import { noMissingTranslations } from "./no-missing-translations.ts";
import { noManifestImport } from "./no-manifest-import.ts";
import { noOverloadCast } from "./no-overload-cast.ts";
import { noPartialAssertions } from "./no-partial-assertions.ts";
import { noSystemEnvironment } from "./no-system-environment.ts";
import { noTestTimeout } from "./no-test-timeout.ts";
import { paddingBeforeReturn } from "./padding-before-return.ts";
import { preventAbbreviations } from "./prevent-abbreviations.ts";
import { requireBlockComment } from "./require-block-comment.ts";
import { requireJsdoc } from "./require-jsdoc.ts";
import { requireRenderPart } from "./require-render-part.ts";
import { styleAttribute } from "./style-attribute.ts";
import { styleFocus } from "./style-focus.ts";
import { styleHover } from "./style-hover.ts";
import { styleMotion } from "./style-motion.ts";
import { styleShorthand } from "./style-shorthand.ts";
import { styleTokens } from "./style-tokens.ts";
import { styleXstyleLast } from "./style-xstyle-last.ts";
import { packageLayout } from "./package-layout.ts";
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
    "deterministic-handler": deterministicHandler,
    "error-message-style": errorMessageStyle,
    "exact-optional": exactOptional,
    "jsdoc-sentence": jsdocSentence,
    "method-signature-style": methodSignatureStyle,
    "no-class-name": noClassName,
    "no-import-alias": noImportAlias,
    "no-index-logic": noIndexLogic,
    "no-inline-config": noInlineConfiguration,
    "no-ambient-effect": noAmbientEffect,
    "no-manifest-import": noManifestImport,
    "message-context": messageContext,
    "no-missing-translations": noMissingTranslations,
    "no-overload-cast": noOverloadCast,
    "no-partial-assertions": noPartialAssertions,
    "no-system-environment": noSystemEnvironment,
    "no-test-timeout": noTestTimeout,
    "padding-before-return": paddingBeforeReturn,
    "package-layout": packageLayout,
    "prevent-abbreviations": preventAbbreviations,
    "require-block-comment": requireBlockComment,
    "require-jsdoc": requireJsdoc,
    "require-render-part": requireRenderPart,
    "style-attribute": styleAttribute,
    "style-focus": styleFocus,
    "style-hover": styleHover,
    "style-motion": styleMotion,
    "style-shorthand": styleShorthand,
    "style-tokens": styleTokens,
    "style-xstyle-last": styleXstyleLast,
    "valid-package-handle": validPackageHandle,
} satisfies Record<string, Rule>;

/** The rules that find what a check compares with other files, off where a file is linted alone. */
export const COMPARED_RULES: ReadonlySet<string> = new Set(["no-missing-translations"]);

/** Mandatory Destack source rules. */
const plugin = {
    meta: { name: "destack" },
    rules,
};

export default plugin;
