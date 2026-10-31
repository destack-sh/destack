import type { JSX } from "@destack/view";
import type { ContentEntry } from "../content/presentation";
import { createDirectory, DirectoryContent } from "../reader/directory";

/** List a collection's entries under its title, filtered by the year the address has. */
export default function Directory(properties: {
    readonly title: string;
    readonly description?: string | undefined;
    readonly entries: readonly ContentEntry[];
    readonly children?: JSX.Element;
}) {
    const directory = createDirectory(() => properties.entries);

    return (
        <DirectoryContent
            title={properties.title}
            description={properties.description}
            directory={directory}
        >
            {properties.children}
        </DirectoryContent>
    );
}
