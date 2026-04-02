export const marketingCopy = {
    channel: "marketing",
    headline: "Launch pages with shared layout chrome",
};

import { renderSharedSummary } from "../chunks/metadata-61dbede3.js";
import { sharedMetadata } from "../chunks/metadata-61dbede3.js";
console.log(renderSharedSummary(marketingCopy.channel, marketingCopy.headline));
console.log(sharedMetadata.channel, sharedMetadata.kind);
