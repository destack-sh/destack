import type { PageSource } from "../content/source";
import { createPageSourceCommands, SourceActions } from "../reader/source";

/** Offer the page's Markdown and text sources to copy and open. */
export default function Source(properties: { readonly source: PageSource }) {
    return <SourceActions commands={createPageSourceCommands(properties.source)} />;
}
