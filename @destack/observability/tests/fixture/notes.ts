import { ids, notes } from "./observability.ts";
import { type DatabaseConnection } from "@destack/db";
import { alertRule, type AlertRuleDefinition } from "../../src/object/index.ts";
import { graph } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { Digest, present, schema } from "@destack/schema";
import type { OtlpEmitter, OtlpReceiver } from "@destack/telemetry/otlp";
import { installationRevision } from "@destack/space/object";

/** The note module's source: a class whose rename method fails, and the module's path. */
export const NOTE_SOURCE = [
    "export class Note {",
    "    rename(title: string) {",
    "        return check(title);",
    "    }",
    "}",
].join("\n");

/** The digest of the notes build's manifest. */
export const NOTES_MANIFEST = Digest.parse("b".repeat(64));

/** The notes workload's instance running the notes build, as its machine verified it. */
export const NOTES_EMITTER: OtlpEmitter = {
    scope: ids.space,
    installation: ids.notes,
    instance: ids.instance,
    build: NOTES_MANIFEST,
};

/** Encode text as UTF-8. */
export function encode(value: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(value);
}

/** Open the notes build: a workload whose first generated line maps into `Note.rename`, an object method declaration, and the alert rules it declares. */
export async function openNotesBuild(
    rules: Readonly<Record<string, AlertRuleDefinition>> = {},
): Promise<BuildReader> {
    // keep the source, the workload's source map and the note module's graph file
    const files = new Map<string, Uint8Array<ArrayBuffer>>();
    files.set("src/note.ts", encode(NOTE_SOURCE));
    files.set(
        "output/bun/workload.js.map",
        encode(
            JSON.stringify({
                version: 3,
                sources: ["../../src/note.ts"],
                names: ["rename"],
                // generated 1:0 → note.ts 3:8, inside rename
                mappings: "AAEQA",
            }),
        ),
    );
    const module = await notesModule(rules);
    files.set(`graph/${module.digest}.json`, module.bytes);

    // list them in the manifest
    const listed = async (path: string, value: unknown) => {
        const bytes = encode(JSON.stringify(value));
        files.set(path, bytes);

        return PackageFile.describe(path, "application/json", bytes);
    };
    const manifest = PackageManifest.parse({
        formatVersion: 1,
        package: notes,
        language: "typescript",
        lists: {
            dependencies: await listed("manifest/dependencies.json", {}),
            files: await listed("manifest/files.json", []),
            sourceMaps: await listed("manifest/sourceMaps.json", [
                {
                    generated: "output/bun/workload.js",
                    map: "output/bun/workload.js.map",
                },
            ]),
            graph: await listed("manifest/graph.json", {
                modules: { "src/note.ts": module.digest },
            }),
        },
        outputs: {},
    });

    return new BuildReader(manifest, async (path) =>
        new Blob([present(files.get(path), `the notes build's file ${path}`)]).stream(),
    );
}

/** Build the note module's graph file: the `Note` class, its `rename` method declaration, and each alert rule. */
async function notesModule(rules: Readonly<Record<string, AlertRuleDefinition>>) {
    return graph.Module.file({
        path: "src/note.ts",
        digest: await Digest.of(encode(NOTE_SOURCE)),
        imports: [],
        exports: [],
        symbols: [
            {
                moniker: at("Note"),
                kind: "class",
                source: { file: "src/note.ts", start: 0, end: NOTE_SOURCE.length },
                signature: "class Note",
                isExported: true,
            },
            {
                moniker: at("Note.rename"),
                kind: "method",
                source: { file: "src/note.ts", start: lineAt(2), end: lineAt(5) - 1 },
                signature: "rename(title: string)",
                isExported: true,
            },
            ...Object.keys(rules).map((name) => ({
                moniker: at(name),
                kind: "variable" as const,
                source: { file: "src/note.ts", start: 0, end: 0 },
                signature: `const ${name}`,
                isExported: true,
            })),
        ],
        declarations: [
            {
                moniker: `${at("Note.rename")}:method`,
                symbol: at("Note"),
                kind: "method",
                package: ids.package,
                name: "rename",
                description: {},
            },
            ...Object.entries(rules).map(([name, rule]) => ({
                moniker: `${at(name)}:alert-rule`,
                symbol: at(name),
                kind: "alert-rule",
                package: alertRule.package.id,
                name,
                description: rule,
            })),
        ],
        edges: [],
    });
}

/** Record the notes revision built from the notes manifest at a time, returning its identifier. */
export async function recordNotesRevision(
    database: DatabaseConnection,
    now = Date.now(),
    manifest: string = NOTES_MANIFEST,
) {
    const id = schema.identifier("installation-revision").parse(installationRevision.generateId());
    await database.insert(installationRevision.table).values({
        id,
        scope: ids.space,
        installationId: ids.notes,
        build: {
            kind: "release",
            version: notes.version,
            manifest,
        },
        views: {},
        settings: [],
        digest: String(now).padStart(64, "c"),
        createdAt: now,
        updatedAt: now,
    });

    return id;
}

/** An attribute value as a test writes it. */
type Value = string | number | boolean | readonly string[];

/** A log record of the notes workload, timed in Unix milliseconds. */
export interface NotesRecord {
    /** When it was emitted. */
    readonly time: number;
    /** Its event name. */
    readonly name: string;
    /** Its severity, absent for none. */
    readonly severity?: number;
    /** Its body, absent for none. */
    readonly body?: string;
    /** Its attributes. */
    readonly attributes?: Readonly<Record<string, Value>>;
    /** The trace it was emitted in. */
    readonly trace?: string;
    /** The span it was emitted in. */
    readonly span?: string;
}

/** A span of the notes workload, timed in Unix milliseconds. */
export interface NotesSpan {
    /** The trace. */
    readonly trace: string;
    /** The span. */
    readonly span: string;
    /** The parent span, absent for a root. */
    readonly parent?: string;
    /** The span's name. */
    readonly name: string;
    /** When it started. */
    readonly start: number;
    /** When it ended. */
    readonly end: number;
    /** Its OTLP status code. */
    readonly status?: 0 | 1 | 2;
    /** Its attributes. */
    readonly attributes?: Readonly<Record<string, Value>>;
    /** The events it recorded, such as exceptions. */
    readonly annotations?: readonly {
        readonly name: string;
        readonly time: number;
        readonly attributes: Readonly<Record<string, Value>>;
    }[];
}

/** Write attributes as OTLP writes them. */
function otlpAttributes(attributes: Readonly<Record<string, Value>> = {}) {
    return Object.entries(attributes).map(([key, value]) => ({
        key,
        value: otlpValue(value),
    }));
}

/** Write one attribute value as OTLP writes it, a list as an array of strings. */
function otlpValue(value: Value) {
    // write a scalar as its typed value
    if (typeof value === "string") {
        return { stringValue: value };
    } else if (typeof value === "number") {
        return { intValue: String(value) };
    } else if (typeof value === "boolean") {
        return { boolValue: value };
    }

    return { arrayValue: { values: value.map((item) => ({ stringValue: item })) } };
}

/** Write milliseconds as OTLP nanoseconds. */
function nanoseconds(time: number): string {
    return String(BigInt(time) * 1_000_000n);
}

/** An OTLP/JSON logs export of the notes workload. */
export function logsExport(records: readonly NotesRecord[]) {
    return {
        resourceLogs: [
            {
                resource: { attributes: otlpAttributes({ "service.name": notes.name }) },
                scopeLogs: [
                    {
                        scope: { name: notes.name, version: notes.version },
                        logRecords: records.map((record) => ({
                            timeUnixNano: nanoseconds(record.time),
                            ...(record.severity === undefined
                                ? {}
                                : { severityNumber: record.severity }),
                            eventName: record.name,
                            ...(record.body === undefined
                                ? {}
                                : { body: { stringValue: record.body } }),
                            attributes: otlpAttributes(record.attributes),
                            ...(record.trace === undefined ? {} : { traceId: record.trace }),
                            ...(record.span === undefined ? {} : { spanId: record.span }),
                        })),
                    },
                ],
            },
        ],
    };
}

/** An OTLP/JSON traces export of the notes workload. */
export function tracesExport(spans: readonly NotesSpan[]) {
    return {
        resourceSpans: [
            {
                resource: { attributes: otlpAttributes({ "service.name": notes.name }) },
                scopeSpans: [
                    {
                        scope: { name: notes.name, version: notes.version },
                        spans: spans.map((span) => ({
                            traceId: span.trace,
                            spanId: span.span,
                            ...(span.parent === undefined ? {} : { parentSpanId: span.parent }),
                            name: span.name,
                            startTimeUnixNano: nanoseconds(span.start),
                            endTimeUnixNano: nanoseconds(span.end),
                            attributes: otlpAttributes(span.attributes),
                            events: (span.annotations ?? []).map((annotation) => ({
                                name: annotation.name,
                                timeUnixNano: nanoseconds(annotation.time),
                                attributes: otlpAttributes(annotation.attributes),
                            })),
                            status: { code: span.status ?? 0 },
                        })),
                    },
                ],
            },
        ],
    };
}

/** Export exception log records as the notes workload's instance, as its machine relays them. */
export async function exportLogs(
    receiver: OtlpReceiver,
    records: readonly {
        readonly time: number;
        readonly attributes: Readonly<Record<string, Value>>;
        readonly trace?: string;
        readonly span?: string;
    }[],
): Promise<void> {
    await receiver.receive(
        NOTES_EMITTER,
        "logs",
        logsExport(records.map((record) => ({ ...record, name: "exception", severity: 17 }))),
    );
}

/** A logs export of the notes package whose resource names one attribute. */
export function claimingLogs(key: string, value: string) {
    return {
        resourceLogs: [
            {
                resource: { attributes: [{ key, value: { stringValue: value } }] },
                scopeLogs: [
                    {
                        scope: { name: notes.name, version: notes.version },
                        logRecords: [{ timeUnixNano: "1", eventName: "note.saved" }],
                    },
                ],
            },
        ],
    };
}

/** A failure of `Note.rename` as the notes workload captures it, for a person. */
export function renameFailure(time: number, person: string) {
    return {
        time,
        attributes: {
            "error.type": "RangeError",
            "exception.type": "RangeError",
            "exception.message": "title is too long\nat most 500 characters",
            "exception.stacktrace":
                "RangeError: title is too long\n    at rename (file:///srv/output/bun/workload.js:1:1)",
            "exception.escaped": true,
            "enduser.id": person,
        },
    };
}

/** Name a symbol of the note module. */
function at(name: string): string {
    return graph.Moniker.of({ packageId: ids.package, module: "src/note.ts", name });
}

/** Find the offset of a line of the note module's source. */
function lineAt(line: number): number {
    return NOTE_SOURCE.split("\n")
        .slice(0, line - 1)
        .reduce((offset, text) => offset + text.length + 1, 0);
}
