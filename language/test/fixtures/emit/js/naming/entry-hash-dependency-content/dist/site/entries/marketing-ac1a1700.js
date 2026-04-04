export const marketingCopy = {
    channel: "marketing",
    headline: "Launch pages with shared layout chrome",
};

import { renderSharedSummary } from "../chunks/chunk-9b093383.js";
import { sharedMetadata } from "../chunks/chunk-9b093383.js";
console.log(renderSharedSummary(marketingCopy.channel, marketingCopy.headline));
console.log(sharedMetadata.channel, sharedMetadata.kind);
