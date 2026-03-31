import { marketingCopy } from "./sections/marketing.ts";
import { renderSharedSummary } from "./shared/summary.ts";
import { sharedMetadata } from "./shared/metadata.ts";

console.log(renderSharedSummary(marketingCopy.channel, marketingCopy.headline));
console.log(sharedMetadata.channel, sharedMetadata.kind);
