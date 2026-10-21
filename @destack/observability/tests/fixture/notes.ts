import type { DatabaseConnection } from "@destack/db";
import { StackFrame, Symbolicator } from "@destack/package/stack";
import { FRAMES_ATTRIBUTE } from "@destack/telemetry/otlp";
import { graph } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { Digest, type Identifier, present } from "@destack/schema";
import { installationRevision } from "@destack/stack/object";
import { insertRelease } from "@destack/stack/test";
import { EMITTER_ATTRIBUTES } from "../../src/otlp/index.ts";
import { ids, notes } from "./space.ts";

/** The digest the notes build's revisions record of its manifest. */
export const NOTES_MANIFEST = Digest.parse(
    "8bdf50d6d97eea3aeea1819f64e41b24dfc909ad0d1e5e6edfd7c7ed48b24bd1",
);

/** The note module's source: a class whose rename method fails, and the module's path. */
export const NOTE_SOURCE = [
    "export class Note {",
    "    rename(title: string) {",
    "        return check(title);",
    "    }",
    "}",
].join("\n");

/** Encode text as UTF-8. */
export function encode(value: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(value);
}

/** Open the notes build: a workload whose source map points into `Note.rename`. */
export async function openNotesBuild(): Promise<BuildReader> {
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
    const module = await notesModule();
    files.set(`graph/${module.digest}.json`, module.bytes);

    // list them in the manifest
    const listed = async (path: string, value: unknown) => {
        const bytes = encode(JSON.stringify(value));
        files.set(path, bytes);

        return PackageFile.describe(path, "application/json", bytes);
    };
    // list every file beside the lists
    const distributed = await Promise.all(
        [...files].map(([path, bytes]) =>
            PackageFile.describe(
                path,
                path.endsWith(".ts") ? "text/plain" : "application/json",
                bytes,
            ),
        ),
    );
    const manifest = PackageManifest.parse({
        formatVersion: 1,
        package: notes,
        language: "typescript",
        lists: {
            dependencies: await listed("manifest/dependencies.json", {}),
            files: await listed("manifest/files.json", distributed),
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

/** Build the note module's graph file: `Note` and its `rename` method. */
async function notesModule() {
    return graph.Module.file({
        path: "src/note.ts",
        digest: await Digest.of(encode(NOTE_SOURCE)),
        imports: [],
        exports: [],
        declarations: [
            {
                moniker: `${at("Note.rename")}:method`,
                symbol: at("Note"),
                kind: "method",
                kindPackageId: ids.package,
                name: "rename",
                description: {},
            },
        ],
        edges: [],
    });
}

/** Record a revision of the notes installation running the notes build, made at a time. */
export async function recordNotesRevision(
    database: DatabaseConnection,
    now: number,
): Promise<Identifier<"installation-revision">> {
    // keep the release as the space's build, and the revision following it
    const release = { package: notes.id, version: notes.version, manifest: NOTES_MANIFEST };
    const pulled = await insertRelease(database, ids.space, release, [], now);
    const id = installationRevision.generateId();
    await database.insert(installationRevision.table).values({
        id,
        scope: ids.space,
        installationId: ids.notes,
        buildId: pulled.id,
        dependencies: [],
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

/** The resource attributes the machine sets on the notes instance's exports it forwards. */
export const NOTES_RESOURCE = {
    "service.name": notes.name,
    [EMITTER_ATTRIBUTES.installation]: ids.notes,
    [EMITTER_ATTRIBUTES.instance]: ids.instance,
    [EMITTER_ATTRIBUTES.build]: NOTES_MANIFEST,
};

/** An OTLP/JSON logs export of the notes workload under a resource. */
export function logsExport(
    records: readonly NotesRecord[],
    resource: Readonly<Record<string, Value>> = NOTES_RESOURCE,
) {
    return {
        resourceLogs: [
            {
                resource: { attributes: otlpAttributes(resource) },
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

/** An OTLP/JSON traces export of the notes workload under a resource. */
export function tracesExport(
    spans: readonly NotesSpan[],
    resource: Readonly<Record<string, Value>> = NOTES_RESOURCE,
) {
    return {
        resourceSpans: [
            {
                resource: { attributes: otlpAttributes(resource) },
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

/** A logs export of exception records of the notes workload. */
export function exceptionsExport(
    records: readonly {
        readonly time: number;
        readonly attributes: Readonly<Record<string, Value>>;
        readonly trace?: string;
        readonly span?: string;
    }[],
) {
    return logsExport(records.map((record) => ({ ...record, name: "exception", severity: 17 })));
}

/** The stack trace of a failure of `Note.rename`, as the notes workload's runtime writes it. */
const RENAME_STACK =
    "RangeError: title is too long\n    at rename (file:///srv/output/bun/workload.js:1:1)";

/** The frames of that failure, symbolicated through the notes build as its runtime exports them. */
const RENAME_FRAMES = JSON.stringify(
    await new Symbolicator(await openNotesBuild()).frames(StackFrame.read(RENAME_STACK)),
);

/** A failure of `Note.rename` as the notes workload captures and exports it, for a person. */
export function renameFailure(time: number, person: string) {
    return {
        time,
        attributes: {
            "error.type": "RangeError",
            "exception.type": "RangeError",
            "exception.message": "title is too long\nat most 500 characters",
            "exception.stacktrace": RENAME_STACK,
            [FRAMES_ATTRIBUTE]: RENAME_FRAMES,
            "exception.escaped": true,
            "enduser.id": person,
        },
    };
}

/** Name a symbol of the note module. */
function at(name: string): string {
    return graph.Moniker.of({ packageId: ids.package, module: "src/note.ts", name });
}
