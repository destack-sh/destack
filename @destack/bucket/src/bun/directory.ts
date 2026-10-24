import { dirname } from "node:path";
import type { FileSystem } from "@destack/environment";

/** Flush a directory's entries before publishing references to its files. */
export async function syncDirectory(directory: string, files: FileSystem): Promise<void> {
    await using handle = await files.open(directory, { read: true });
    await handle.sync();
}

/** Create a directory and persist each newly created entry through its existing parent. */
export async function createDurably(directory: string, files: FileSystem): Promise<void> {
    // find the nearest existing ancestor, then create the directory and its missing parents
    let existing = directory;
    while (!(await files.exists(existing))) {
        existing = dirname(existing);
    }
    if (existing === directory) {
        return;
    }
    await files.mkdir(directory, { recursive: true, mode: 0o700 });

    // sync each created entry's parent, from the directory up to the existing ancestor
    for (let current = directory; current !== existing; current = dirname(current)) {
        await syncDirectory(dirname(current), files);
    }
}
