import { buildSourceMapLabel } from "./labels.ts";
import { renderSourceMapCard } from "./render/card.ts";
import { runExternalSourceMapped } from "./sourcemapped.ts";

runExternalSourceMapped(() => {
    const sourceMapLabel = buildSourceMapLabel("merged");

    return renderSourceMapCard("merged", sourceMapLabel);
});
