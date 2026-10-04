import type { graph, Package } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import type { DescriptionReference, PackageOutput } from "@destack/package/manifest";
import type { TestDeclaration } from "@destack/test/inspect";
import { stringifyInspection } from "./serialization.ts";
import { BuildError } from "../error/index.ts";
import type { JsonValue } from "@destack/schema";

/** The lists a manifest refers to, each kept in `manifest/<list>.json`. */
export const MANIFEST_LISTS = ["dependencies", "files", "sourceMaps", "graph"] as const;

/** Shared descriptions collected across build targets. */
export interface ManifestDescription {
    /** The graph file of each module. */
    graph: graph.Module[];
    /** Unique static test declarations. */
    tests: TestDeclaration[];
    /** The package defining static test descriptions. */
    testPackage: Package;
    /** The test indices each output selects, by output name. */
    selections: Map<string, number[]>;
}

/** Serialize the test declarations and select each output's tests. */
export async function serializeTests(
    source: ManifestDescription,
    outputs: Record<string, PackageOutput>,
): Promise<{
    tests: DescriptionReference | undefined;
    files: Map<string, Uint8Array<ArrayBuffer>>;
}> {
    // keep test declarations in a separate file
    const files = new Map<string, Uint8Array<ArrayBuffer>>();
    let tests: DescriptionReference | undefined;
    if (source.tests.length) {
        const file = "manifest/tests.json";
        const bytes = encodeDescription(source.tests);
        tests = {
            package: source.testPackage,
            file: await PackageFile.describe(file, "application/json", bytes),
        };
        files.set(file, bytes);
    }

    // select each output's tests
    for (const [name, selected] of source.selections) {
        const output = outputs[name];
        if (output === undefined) {
            throw new BuildError("BUILD_FAILED", `selection has no output: ${name}`);
        }
        output.tests = selected;
    }

    return { tests, files };
}

/** Encode descriptions as readable JSON without implicit value conversions. */
export function encodeDescription(value: JsonValue): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(`${stringifyInspection(value, 4)}\n`);
}
