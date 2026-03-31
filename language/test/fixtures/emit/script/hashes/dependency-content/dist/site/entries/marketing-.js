export const marketingCopy = {
    channel: "marketing",
    headline: "Launch pages with shared layout chrome",
};

import { renderSharedSummary } from "../chunks/metadata-.js";
import { sharedMetadata } from "../chunks/metadata-.js";
console.log(renderSharedSummary(marketingCopy.channel, marketingCopy.headline));
console.log(sharedMetadata.channel, sharedMetadata.kind);
