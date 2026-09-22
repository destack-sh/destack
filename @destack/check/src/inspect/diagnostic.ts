import { defineSchema, schema } from "@destack/schema";

/** A checker diagnostic with source positions provided by the selected tool. */
export const Diagnostic = defineSchema(
    schema.object({
        /** The rule identifier, absent for parser and tool failures. */
        code: schema.string().optional(),
        /** The user-facing explanation. */
        message: schema.string(),
        /** Suggested correction supplied by the tool. */
        help: schema.string().optional(),
        /** Additional context supplied by the tool. */
        note: schema.string().optional(),
        /** Rule documentation. */
        url: schema.string().optional(),
        /** The diagnostic severity. */
        severity: schema.enum(["error", "warning", "advice"]),
        /** The source filename. */
        filename: schema.string(),
        /** Annotated source spans; offsets and lengths use UTF-8 bytes. */
        labels: schema.array(
            schema.object({
                /** Optional explanation attached to the span. */
                label: schema.string().optional(),
                /** Source coordinates reported by Oxc, with one-based line and column. */
                span: schema.object({
                    offset: schema.number(),
                    length: schema.number(),
                    line: schema.number(),
                    column: schema.number(),
                }),
            }),
        ),
    }),
);

/** A checker diagnostic. */
export type Diagnostic = schema.Infer<typeof Diagnostic>;

/** The complete diagnostics from an Oxc invocation. */
export const CheckResult = defineSchema(
    schema.object({
        /** All reported diagnostics. */
        diagnostics: schema.array(Diagnostic),
    }),
);

/** The complete diagnostics from a source check. */
export type CheckResult = schema.Infer<typeof CheckResult>;
