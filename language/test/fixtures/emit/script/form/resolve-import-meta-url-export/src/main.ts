import { buildPreviewLabel, buildPreviewSourceUrl } from "./meta/build.ts";

export const url = import.meta.url;
export const meta = import.meta;
export const previewSourceUrl = buildPreviewSourceUrl("cards");
export const previewLabel = buildPreviewLabel("cards");
