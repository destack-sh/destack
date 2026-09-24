import type { Package } from "@destack/package";
import { describeFile } from "@destack/package/file";
import type { PackageManifest, PackageOutput, FileDescription } from "@destack/package/manifest";
import type { ModuleDescription } from "@destack/package/code";
import type { TestDeclaration } from "@destack/test/inspect";
import type { DeclarationDescription } from "@destack/package/inspect";
import { stringifyInspection } from "./serialization.ts";
import { BuildError } from "../error/index.ts";

/** Shared descriptions collected across build targets. */
export interface ManifestDescription {
    /** Unique module descriptions. */
    modules: ModuleDescription[];
    /** Unique domain declarations. */
    declarations: DeclarationDescription[];
    /** Unique static test declarations. */
    tests: TestDeclaration[];
    /** The package defining static test descriptions. */
    testPackage: Package;
    /** The package defining module descriptions. */
    modulePackage: Package;
    /** Declaration and test indices selected by each output. */
    selections: Map<string, { modules: number[]; declarations: number[]; tests: number[] }>;
}

/** Serialize domain collections and independently readable code descriptions. */
export async function serializeDescriptions(
    source: ManifestDescription,
    outputs: Record<string, PackageOutput>,
) {
    // group descriptions by package
    const files = new Map<string, Uint8Array<ArrayBuffer>>();
    const descriptions: PackageManifest["descriptions"] = {};
    const groups = new Map<string, { package: Package; values: unknown[] }>();
    const references: { group: string; index: number }[] = [];

    // collect declarations by the exact domain package defining them
    for (const declaration of source.declarations) {
        const owner = declaration.constructor.package;
        const key = `${owner.id}@${owner.version}`;
        let group = groups.get(key);
        if (!group) {
            group = { package: owner, values: [] };
            groups.set(key, group);
        }
        references.push({ group: key, index: group.values.length });
        group.values.push(declaration);
    }

    // retain test descriptions as their domain's collection
    const testKey = `${source.testPackage.id}@${source.testPackage.version}`;
    if (source.tests.length) {
        groups.set(testKey, { package: source.testPackage, values: source.tests });
    }

    // use short names unless package names or versions would collide
    const names = new Map<string, number>();
    for (const group of groups.values()) {
        const name = group.package.name.split("/").at(-1)!;
        const count = names.get(name);
        names.set(name, count === undefined ? 1 : count + 1);
    }
    const domains = new Map<string, string>();
    for (const [key, group] of groups) {
        const name = group.package.name.split("/").at(-1)!;
        const isReserved = ["dependencies", "files", "sourceMaps"].includes(name);
        const domain =
            names.get(name) === 1 && !isReserved ? name : `${name}-${encodeURIComponent(key)}`;
        const file = `manifest/${domain}.json`;
        domains.set(key, domain);
        const bytes = encodeDescription(group.values);
        descriptions[domain] = {
            package: group.package,
            file: await describeFile(file, "application/json", bytes),
        };
        files.set(file, bytes);
    }

    // translate output selections into collection-local indices
    for (const [name, selected] of source.selections) {
        const collections = outputs[name].descriptions;
        for (const index of selected.declarations) {
            const reference = references[index];
            const domain = domains.get(reference.group)!;
            (collections[domain] ??= []).push(reference.index);
        }
        if (selected.tests.length) {
            collections[domains.get(testKey)!] = selected.tests;
        }
    }

    // identify paths with target-specific descriptions
    const counts = new Map<string, number>();
    for (const module of source.modules) {
        const count = counts.get(module.path);
        counts.set(module.path, count === undefined ? 1 : count + 1);
    }
    const owners = new Map<number, string[]>();
    for (const name of [...source.selections.keys()].sort()) {
        for (const index of source.selections.get(name)!.modules) {
            const names = owners.get(index) ?? [];
            names.push(name);
            owners.set(index, names);
        }
    }

    // preserve source paths and qualify only distinct variants by output name
    const modules = new Map<string, FileDescription[]>();
    for (const [index, module] of source.modules.entries()) {
        let directory = "manifest/files";
        if (counts.get(module.path)! > 1) {
            const output = owners.get(index)?.[0];
            if (!output) {
                throw new BuildError(
                    "BUILD_FAILED",
                    `module variant has no output: ${module.path}`,
                );
            }
            directory = `manifest/files/output/${output}`;
        }
        const description = `${directory}/${module.path}.json`;
        if (files.has(description)) {
            throw new BuildError("BUILD_FAILED", `conflicting module descriptions: ${description}`);
        }
        const bytes = encodeDescription(module);
        files.set(description, bytes);
        const variants = modules.get(module.path) ?? [];
        variants.push({
            package: source.modulePackage,
            kind: "module",
            outputs: owners.get(index)!,
            file: await describeFile(description, "application/json", bytes),
        });
        modules.set(module.path, variants);
    }

    return { modules, descriptions, files };
}

/** Encode descriptions as readable JSON without implicit value conversions. */
export function encodeDescription(value: unknown): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(`${stringifyInspection(value, 4)}\n`);
}
