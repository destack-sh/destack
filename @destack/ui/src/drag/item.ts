/** The type of a drag of files from outside the page, as the platform calls it. */
export const FILES_TYPE = "Files";

/** The type of a drag of links. */
export const LINK_TYPE = "text/uri-list";

/** The type of a drag of text. */
export const TEXT_TYPE = "text/plain";

/** Something a drag brings from outside the page. */
export type DragItem =
    | {
          /** A file. */
          readonly kind: "file";
          /** The file. */
          readonly file: File;
      }
    | {
          /** A folder. */
          readonly kind: "directory";
          /** The folder, which lists what is in it. */
          readonly entry: FileSystemDirectoryEntry;
      }
    | {
          /** Text, such as a link. */
          readonly kind: "text";
          /** Its media type. */
          readonly type: string;
          /** The text. */
          readonly text: string;
      };

/** The verbs of what a drag brings. */
export const DragItem = {
    /** Read the type of a drag, files before links before text, none for an empty one. */
    typeOf(types: readonly string[]): string | undefined {
        return [FILES_TYPE, LINK_TYPE, TEXT_TYPE].find((type) => types.includes(type)) ?? types[0];
    },

    /** Read what a drop brings, which the platform tells no sooner. */
    of(transfer: DataTransfer): readonly DragItem[] {
        // read the files and folders, else each text
        const files = [...transfer.items].flatMap((each): DragItem[] => {
            // take a folder by its entry, as it has no bytes
            const entry = each.kind === "file" ? each.webkitGetAsEntry() : null;
            const file = each.kind === "file" ? each.getAsFile() : null;
            if (isDirectory(entry)) {
                return [{ kind: "directory", entry }];
            }

            return file === null ? [] : [{ kind: "file", file }];
        });

        return files.length > 0
            ? files
            : transfer.types
                  .filter((type) => type !== FILES_TYPE)
                  .map((type) => ({ kind: "text", type, text: transfer.getData(type) }));
    },
};

/** Report whether an entry is a folder. */
function isDirectory(entry: FileSystemEntry | null): entry is FileSystemDirectoryEntry {
    return entry?.isDirectory === true;
}
