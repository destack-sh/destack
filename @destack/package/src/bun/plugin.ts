/// <reference types="bun" />
import {
    DECLARED_MODULE,
    DECLARED_PASSTHROUGH,
    type ModuleTransformer,
} from "../transform/transform.ts";

/** Inject Destack module metadata while Bun loads package sources, stamped once per text. */
export function modulePlugin(transformer: ModuleTransformer): Bun.BunPlugin {
    return {
        name: "destack-module",
        setup(build) {
            // provide the declaration recorder as its pass-through
            build.module(DECLARED_MODULE, () => ({ contents: DECLARED_PASSTHROUGH, loader: "js" }));

            // stamp package sources, since runtime loads need contents and bundles take the rest
            const isBundling = build.config !== undefined;
            build.onLoad({ filter: /\.[cm]?tsx?$/u }, async ({ path }) => {
                // stamp the module's source, passing unchanged sources on to later bundler plugins
                const code = await transformer.environment.files.readTextFile(path);
                const result = await transformer.transform(code, path);
                if (!result && isBundling) {
                    return undefined;
                }
                const loader = /\.[cm]?tsx$/u.test(path) ? "tsx" : "ts";
                if (!result) {
                    return { contents: code, loader };
                }

                // map stamped sources back to their authored lines
                return {
                    contents: `${result.code}\n//# sourceMappingURL=${result.map.toUrl()}`,
                    loader,
                };
            });
        },
    };
}
