import { join } from "node:path";
import type { FileSystem } from "@destack/environment";
import { present } from "@destack/schema";
import type { Diagnostic } from "../diagnostic/diagnostic.ts";
import { CheckError } from "../error/index.ts";
import { Workspace } from "../workspace/workspace.ts";
import { readManifest } from "../check/manifest.ts";

/** The scope of the packages the architecture lays out. */
const SCOPE = "@destack/";

/** The directory of a package the architecture lays out: one directly in the scope's directory. */
const SCOPE_DIRECTORY = /^@destack\/[^/]+$/u;

/** A layer's heading: its name, numbered for the platform's layers. */
const LAYER_HEADING = /^## (?:\d+ )?(.+)$/u;

/** A package's row: its linked name, its description and its prior art. */
const PACKAGE_ROW = /^\| \[`(@[^`]+)`\]\([^)]*\) \| (.+?) \| [^|]* \|$/u;

/** A workspace package as the architecture checks it. */
export interface ArchitecturePackage {
    /** What the package does, from its package.json. */
    readonly description: string;
    /** The packages it depends on at run time, by name. */
    readonly dependencies: readonly string[];
}

/** A package's row in a layer's table. */
interface ArchitectureRow {
    /** The package's name. */
    readonly name: string;
    /** What the row says the package does. */
    readonly description: string;
    /** The zero-based line of the row. */
    readonly line: number;
    /** The character offset where the row starts. */
    readonly offset: number;
}

/** A layer and its packages' rows. */
interface ArchitectureLayer {
    /** The layer's name. */
    readonly name: string;
    /** The rows of its packages. */
    readonly rows: readonly ArchitectureRow[];
}

/** A rule failure before it is placed in its file. */
interface Finding {
    /** The rule identifier. */
    readonly code: "member" | "layer" | "description";
    /** The user-facing explanation. */
    readonly message: string;
    /** The zero-based line. */
    readonly line: number;
    /** The character offset. */
    readonly offset: number;
}

/** The repository's layers as ARCHITECTURE.md declares them, the highest first. */
export class Architecture {
    /** The layers, the highest first. */
    readonly layers: readonly ArchitectureLayer[];

    /** Create an architecture from its layers. */
    private constructor(layers: readonly ArchitectureLayer[]) {
        this.layers = layers;
    }

    /** Read the layers from ARCHITECTURE.md's sections and their package rows. */
    static parse(text: string): Architecture {
        // open a layer at each numbered heading and add each package row to the open one
        const layers: { name: string; rows: ArchitectureRow[] }[] = [];
        let offset = 0;
        for (const [line, row] of text.split("\n").entries()) {
            const heading = LAYER_HEADING.exec(row);
            const listed = PACKAGE_ROW.exec(row);
            const layer = layers.at(-1);
            if (heading?.[1] !== undefined) {
                layers.push({ name: heading[1], rows: [] });
            } else if (
                listed?.[1] !== undefined &&
                listed[2] !== undefined &&
                layer !== undefined
            ) {
                layer.rows.push({ name: listed[1], description: listed[2], line, offset });
            }
            offset += row.length + 1;
        }

        return new Architecture(layers);
    }

    /** Read the workspace's packages under the scope, with their run-time dependencies. */
    static async packages(
        directory: string,
        files: FileSystem,
    ): Promise<ReadonlyMap<string, ArchitecturePackage>> {
        // read each member's package.json under the scope
        const workspace = await Workspace.read(directory, files);
        if (workspace === undefined) {
            throw CheckError.of(
                "CONFIGURATION_INVALID",
                `architecture '${directory}' is invalid: it has no workspace lockfile`,
            );
        }
        const members = workspace.members.filter((member) =>
            SCOPE_DIRECTORY.test(member.directory),
        );
        const read = await Promise.all(
            members.map(async (member) => {
                // read its description and its dependencies under the scope, leaving out optional peers
                const folder = join(workspace.root, member.directory);
                const manifest = present(
                    await readManifest(folder, files),
                    `the package.json of ${folder}`,
                );
                const peers = Object.keys(manifest.peerDependencies ?? {}).filter(
                    (name) => manifest.peerDependenciesMeta?.[name]?.optional !== true,
                );
                const dependencies = [
                    ...Object.keys(manifest.dependencies ?? {}),
                    ...peers,
                    ...Object.keys(manifest.optionalDependencies ?? {}),
                ].filter((name) => name.startsWith(SCOPE));

                return [
                    present(manifest.name, `the name of ${folder}`),
                    {
                        description: present(manifest.description, `the description of ${folder}`),
                        dependencies,
                    },
                ] as const;
            }),
        );

        return new Map(read);
    }

    /** Check the layers against the packages as a file's diagnostics. */
    check(
        packages: ReadonlyMap<string, ArchitecturePackage>,
        filename: string,
        text: string,
    ): Diagnostic[] {
        // place every listed package in its layer, the highest first
        const findings: Finding[] = [];
        const layerOf = new Map<string, number>();
        for (const [index, layer] of this.layers.entries()) {
            for (const row of layer.rows) {
                // refuse a package listed twice or missing from the workspace
                const known = packages.get(row.name);
                if (layerOf.has(row.name)) {
                    findings.push(
                        Architecture.#at(row, "member", `list '${row.name}' in one layer`),
                    );
                } else if (known === undefined) {
                    findings.push(
                        Architecture.#at(
                            row,
                            "member",
                            `remove '${row.name}': the workspace has no such package`,
                        ),
                    );
                }
                // refuse a row whose description differs from the package's own
                else if (row.description !== known.description) {
                    findings.push(
                        Architecture.#at(
                            row,
                            "description",
                            `describe '${row.name}' as its package.json does: '${known.description}'`,
                        ),
                    );
                }
                layerOf.set(row.name, index);
            }
        }

        // refuse a package no layer lists, at the top of the file
        for (const name of packages.keys()) {
            if (!layerOf.has(name)) {
                findings.push({
                    code: "member",
                    message: `list '${name}' in its layer`,
                    line: 0,
                    offset: 0,
                });
            }
        }

        // refuse a run-time dependency on a higher layer, which an earlier section lists
        for (const [index, layer] of this.layers.entries()) {
            for (const row of layer.rows) {
                for (const dependency of packages.get(row.name)?.dependencies ?? []) {
                    const above = layerOf.get(dependency);
                    if (above !== undefined && above < index) {
                        const higher = present(this.layers[above], `layer ${above}`).name;
                        findings.push(
                            Architecture.#at(
                                row,
                                "layer",
                                `'${row.name}' in ${layer.name} depends on '${dependency}' in the higher layer ${higher}`,
                            ),
                        );
                    }
                }
            }
        }

        // place each finding in the file with its UTF-8 offset
        return findings.map((finding) => ({
            code: `architecture(${finding.code})`,
            message: finding.message,
            severity: "error" as const,
            filename,
            labels: [
                {
                    span: {
                        offset: new TextEncoder().encode(text.slice(0, finding.offset)).length,
                        length: 1,
                        line: finding.line + 1,
                        column: 1,
                    },
                },
            ],
        }));
    }

    /** Report a rule's failure at a row. */
    static #at(row: ArchitectureRow, code: Finding["code"], message: string): Finding {
        return { code, message, line: row.line, offset: row.offset };
    }
}
