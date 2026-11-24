import type { FilePatch } from "./file.ts";
import { PatchReader } from "./reader.ts";

/** The changes of some files, as git writes them. */
export class Patch {
    /** The changes by file. */
    readonly files: readonly FilePatch[];

    /** Keep the changes of some files. */
    constructor(files: readonly FilePatch[]) {
        this.files = files;
    }

    /** Read a patch in git's format, refusing hunks whose lines do not add up. */
    static read(text: string): Patch {
        return new Patch(new PatchReader(text).files());
    }

    /** Put other changes in place of a file's, or leave the file out. */
    replace(file: FilePatch, next: FilePatch | undefined): Patch {
        return new Patch(
            this.files.flatMap((each) =>
                each !== file ? [each] : next === undefined ? [] : [next],
            ),
        );
    }

    /** Turn the changes around, which takes them back. */
    invert(): Patch {
        return new Patch(this.files.map((file) => file.invert()));
    }

    /** Write the changes in git's format. */
    toString(): string {
        return this.files.map(String).join("");
    }
}
