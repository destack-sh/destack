import type { ContentEntry } from "../content/presentation";
import { createDirectory, DirectoryArchive } from "../reader/directory";

/** Browse a dated collection by publication year, kept in the address. */
export default function Archive(properties: { readonly entries: readonly ContentEntry[] }) {
    return <DirectoryArchive directory={createDirectory(() => properties.entries)} />;
}
