export const docsCopy = { channel: "docs", headline: "Guides for shipping bundled pages" };

import { renderSharedSummary } from "../chunks/metadata-.js";
import { sharedMetadata } from "../chunks/metadata-.js";
console.log(renderSharedSummary(docsCopy.channel, docsCopy.headline));
console.log(sharedMetadata.channel, sharedMetadata.kind);
