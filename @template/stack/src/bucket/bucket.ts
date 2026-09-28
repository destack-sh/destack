import { defineBucket } from "@destack/bucket/declare";

/** Shared document and media storage. */
export const files = defineBucket({ name: "files", spec: {} });
