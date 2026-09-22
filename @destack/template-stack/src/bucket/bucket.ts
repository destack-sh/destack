import { defineBucket } from "@destack/storage/declare";

/** Shared document and media storage. */
export const files = defineBucket({ name: "files", spec: {} });
