import type { ModuleMetadata } from "./metadata.ts";

/** Global declarations extended by the Destack build. */
declare global {
    /** Metadata supplied by the module loader and Destack build tooling. */
    interface ImportMeta {
        /** Immutable metadata injected by the Destack build. */
        readonly destack: ModuleMetadata;
    }
}

export {};
