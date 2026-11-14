import { fileURLToPath } from "node:url";
import MagicString from "magic-string";
import type { Plugin } from "vite";

/** The module id an application entry imports first to install the browser baseline. */
export const BASELINE_MODULE = "virtual:destack/baseline";

/** Resolve the browser baseline, and run it first in HTML pages and workers. */
export function baselinePlugin(): Plugin {
    // read whether the build serves, and whether it bundles a worker
    let isServing = false;
    let isWorker = false;

    return {
        name: "destack-baseline",
        enforce: "post",
        configResolved(configuration) {
            isServing = configuration.command === "serve";
            isWorker = configuration.isWorker;
        },
        resolveId(id) {
            return id === BASELINE_MODULE
                ? fileURLToPath(import.meta.resolve("@destack/environment/baseline"))
                : undefined;
        },
        transformIndexHtml() {
            // run the baseline before a page's other scripts
            const source = isServing ? `/@id/${BASELINE_MODULE}` : BASELINE_MODULE;

            return [
                { tag: "script", attrs: { type: "module", src: source }, injectTo: "head-prepend" },
            ];
        },
        transform(code, id) {
            // import the baseline first in a worker's entry, built or served
            const isWorkerEntry = isServing
                ? id.includes("worker_file")
                : isWorker && this.getModuleInfo(id)?.isEntry === true;
            if (!isWorkerEntry) {
                return undefined;
            }
            const prepended = new MagicString(code).prepend(`import "${BASELINE_MODULE}";\n`);

            return {
                code: prepended.toString(),
                map: prepended.generateMap({ hires: true, source: id }),
            };
        },
    };
}
