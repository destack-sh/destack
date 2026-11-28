import type { DeclarationModule } from "@destack/package/build";
import { WORKLOAD_EXPORT } from "./workload.ts";

/** The generated module hosting a workload on its runtime, as a build writes it. */
export const WorkloadModule = {
    /** Write a module hosting a workload after the lines declaring it, with its resources. */
    write(
        lines: readonly string[],
        workload: string,
        resources: readonly { readonly name: string; readonly module: DeclarationModule }[],
        runtime: "bun" | "workerd",
    ): string {
        const bound = `{ ${resources.map(({ name }, index) => `${JSON.stringify(name)}: resource${index}`).join(", ")} }`;

        return [
            runtime === "bun"
                ? 'import { hostWorkload } from "@destack/object/bun";'
                : 'import { hostWorkload } from "@destack/object/cloudflare";',
            ...resources.map(({ module }, index) =>
                WorkloadModule.import(module, `resource${index}`),
            ),
            ...lines,
            `export const ${WORKLOAD_EXPORT} = hostWorkload(${workload}, ${bound});`,
            ...(runtime === "bun" ? [`if (import.meta.main) await ${WORKLOAD_EXPORT}.run();`] : []),
        ].join("\n");
    },

    /** Write an import of a located declaration under a local name. */
    import(module: DeclarationModule, local: string): string {
        return `import { ${module.export} as ${local} } from ${JSON.stringify(module.file)};`;
    },
};
