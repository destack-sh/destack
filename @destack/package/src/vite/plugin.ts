import type { Plugin } from "vite";
import {
    DECLARED_MODULE,
    DECLARED_PASSTHROUGH,
    type ModuleTransformer,
} from "../transform/transform.ts";

/** The resolved id of the declaration recorder, a pass-through outside evaluation. */
const DECLARED_ID = `\0${DECLARED_MODULE}`;

/** Inject module metadata while Vite loads package sources, each module stamped once per build. */
export function modulePlugin(transformer: ModuleTransformer): Plugin {
    return {
        name: "destack-module",
        enforce: "pre",
        resolveId(specifier) {
            return specifier === DECLARED_MODULE ? DECLARED_ID : null;
        },
        load(id) {
            return id === DECLARED_ID ? DECLARED_PASSTHROUGH : null;
        },
        transform: {
            filter: { id: /\.[cm]?tsx?$/u },
            async handler(code, id) {
                // leave virtual modules unchanged, and stamp the others once per text
                const query = id.indexOf("?");
                const path = query === -1 ? id : id.slice(0, query);
                if (id.startsWith("\0")) {
                    return;
                }

                return await transformer.transform(code, path);
            },
        },
    };
}
