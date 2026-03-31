import { docsCopy } from "./sections/docs.ts";
import { renderSharedSummary } from "./shared/summary.ts";
import { sharedMetadata } from "./shared/metadata.ts";

console.log(renderSharedSummary(docsCopy.channel, docsCopy.headline));
console.log(sharedMetadata.channel, sharedMetadata.kind);
