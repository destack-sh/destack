import { defineSchema, schema } from "@destack/schema";
import type { TestCase, TestModule, TestSuite } from "vitest/node";
import {
    AnnotationDescription,
    ArtifactDescription,
    describeAnnotation,
    describeArtifact,
} from "./attachment.ts";

/** A source declaration's position. */
export const LocationDescription = defineSchema(
    schema.object({ line: schema.number().int(), column: schema.number().int() }),
);
/** A reported execution error. */
export const ErrorDescription = defineSchema(
    schema.object({
        name: schema.string().optional(),
        message: schema.string(),
        stack: schema.string().optional(),
    }),
);
/** A test's execution outcome. */
export const ResultDescription = defineSchema(schema.object({
    state: schema.enum(["pending", "passed", "failed", "skipped"]),
    errors: schema.array(ErrorDescription),
    note: schema.string().optional(),
}));
/** Execution measurements supplied by the runner. */
export const DiagnosticDescription = defineSchema(schema.object({
    duration: schema.number(),
    startTime: schema.number(),
    heap: schema.number().optional(),
    slow: schema.boolean(),
    retryCount: schema.number().int(),
    repeatCount: schema.number().int(),
    flaky: schema.boolean(),
}));
/** Captured standard output or standard error. */
export const OutputDescription = defineSchema(
    schema.object({
        type: schema.enum(["stdout", "stderr"]),
        content: schema.string(),
        time: schema.number(),
    }),
);
/** Identity and annotations shared by tests and suites. */
const DeclarationDescription = schema.object({
    id: schema.string(),
    name: schema.string(),
    location: LocationDescription.optional(),
    metadata: schema.record(schema.string(), schema.json()),
    logs: schema.array(OutputDescription),
});

/** A collected case and its current execution result. */
export const TestDescription = defineSchema(DeclarationDescription.extend({
    fullName: schema.string(),
    tags: schema.array(schema.string()),
    mode: schema.enum(["run", "only", "skip", "todo"]),
    result: ResultDescription,
    diagnostic: DiagnosticDescription.optional(),
    annotations: schema.array(AnnotationDescription),
    artifacts: schema.array(ArtifactDescription),
}));
/** A test case's portable description. */
export type TestDescription = schema.Infer<typeof TestDescription>;

/** A collected suite and its child declarations. */
export const SuiteDescription = DeclarationDescription.extend({
    state: schema.enum(["skipped", "pending", "failed", "passed"]),
    errors: schema.array(ErrorDescription),
    tests: schema.array(TestDescription),
    get suites() {
        return schema.array(SuiteDescription);
    },
});
/** A suite's portable description. */
export type SuiteDescription = schema.Infer<typeof SuiteDescription>;

/** A collected module and its current execution results. */
export const ModuleDescription = defineSchema(schema.object({
    id: schema.string(),
    module: schema.string(),
    project: schema.string(),
    state: schema.enum(["queued", "skipped", "pending", "failed", "passed"]),
    errors: schema.array(ErrorDescription),
    metadata: schema.record(schema.string(), schema.json()),
    logs: schema.array(OutputDescription),
    tests: schema.array(TestDescription),
    suites: schema.array(SuiteDescription),
}));
/** A module's portable description. */
export type ModuleDescription = schema.Infer<typeof ModuleDescription>;

/** Describe a collected case without retaining runner objects. */
export function describeTest(test: TestCase): TestDescription {
    const result = test.result();

    return TestDescription.parse({
        id: test.id,
        name: test.name,
        fullName: test.fullName,
        location: test.location,
        mode: test.options.mode,
        tags: test.tags,
        result: {
            state: result.state,
            errors: result.errors?.map(describeError) ?? [],
            note: result.state === "skipped" ? result.note : undefined,
        },
        diagnostic: test.diagnostic(),
        annotations: test.annotations().map(describeAnnotation),
        artifacts: test.artifacts().map(describeArtifact),
        metadata: describeMetadata(test.meta()),
        logs: describeOutput(test),
    });
}

/** Describe a collected suite and its nested cases. */
export function describeSuite(suite: TestSuite): SuiteDescription {
    return {
        id: suite.id,
        name: suite.name,
        location: suite.location,
        state: suite.state(),
        errors: suite.errors().map(describeError),
        metadata: describeMetadata(suite.meta()),
        logs: describeOutput(suite),
        tests: [...suite.children.tests()].map(describeTest),
        suites: [...suite.children.suites()].map(describeSuite),
    };
}

/** Describe a module after collection or execution. */
export function describeModule(module: TestModule): ModuleDescription {
    return {
        id: module.id,
        module: module.moduleId,
        project: module.project.name,
        state: module.state(),
        errors: module.errors().map(describeError),
        metadata: describeMetadata(module.meta()),
        logs: describeOutput(module),
        tests: [...module.children.tests()].map(describeTest),
        suites: [...module.children.suites()].map(describeSuite),
    };
}

/** Retain standard error fields from the runner's serialized errors. */
function describeError(error: { name?: string; message: string; stack?: string }) {
    return { name: error.name, message: error.message, stack: error.stack };
}

/** Copy captured output into the portable log shape. */
function describeOutput(test: TestCase | TestSuite | TestModule) {
    return test.logs().map((log) => ({ type: log.type, content: log.content, time: log.time }));
}

/** Omit absent metadata properties and require JSON values for present properties. */
function describeMetadata(metadata: object) {
    const properties = Object.fromEntries(
        Object.entries(metadata).filter(([, value]) => value !== undefined),
    );

    return schema.record(schema.string(), schema.json()).parse(properties);
}
