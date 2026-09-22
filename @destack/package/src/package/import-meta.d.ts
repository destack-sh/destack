import type { ModuleMetadata } from "./metadata.ts";

declare global {
    /** Metadata supplied by the module loader and Destack build tooling. */
    interface ImportMeta {
        /** Immutable metadata injected by the Destack build. */
        readonly destack: ModuleMetadata;
    }
}

export {};
