/** A range of a text by its offsets. */
export interface Range {
    /** The first offset. */
    readonly from: number;
    /** The offset after the last, the first again for an empty range. */
    readonly to: number;
}

/** A range of a source by its lines. */
export interface LineRange {
    /** The first line. */
    readonly start: number;
    /** The line after the last, the first again for an empty range. */
    readonly end: number;
}

/** A replacement of some text of a source. */
export interface Edit extends Range {
    /** The text put in its place. */
    readonly insert: string;
}

/** Replacements of some text. */
export const Edit = {
    /** List replacements by where they are. */
    list(edits: Edit | readonly Edit[]): readonly Edit[] {
        return "from" in edits ? [edits] : edits.toSorted((one, other) => one.from - other.from);
    },
};

/** The character code of a line break. */
const LINE_BREAK = 10;

/** A text and where each of its lines starts. */
export class Source {
    /** The text. */
    readonly text: string;
    /** The offset each line starts at, the first at zero. */
    readonly starts: Uint32Array;

    /** Keep a text with the starts of its lines. */
    private constructor(text: string, starts: Uint32Array) {
        this.text = text;
        this.starts = starts;
    }

    /** Read a text. */
    static of(text: string): Source {
        // count the lines, then note where each starts
        let count = 1;
        for (let at = text.indexOf("\n"); at !== -1; at = text.indexOf("\n", at + 1)) {
            count += 1;
        }
        const starts = new Uint32Array(count);
        let line = 1;
        for (let at = text.indexOf("\n"); at !== -1; at = text.indexOf("\n", at + 1)) {
            starts[line] = at + 1;
            line += 1;
        }

        return new Source(text, starts);
    }

    /** Whether it ends with a line break, or with nothing. */
    get isClosed(): boolean {
        return this.start(this.starts.length - 1) === this.text.length;
    }

    /** The count of lines, none for what follows a closing line break. */
    get lines(): number {
        return this.isClosed ? this.starts.length - 1 : this.starts.length;
    }

    /** Find the offsets of a line, its line break left out. */
    range(line: number): Range {
        const from = this.start(line);

        return {
            from,
            to: line + 1 < this.starts.length ? this.start(line + 1) - 1 : this.text.length,
        };
    }

    /** Find the offset a line starts at, the text's end past the last. */
    start(line: number): number {
        return this.starts[line] ?? this.text.length;
    }

    /** Read a line without its line break. */
    line(line: number): string {
        const { from, to } = this.range(line);

        return this.text.slice(from, to);
    }

    /** Read some lines with their line breaks. */
    slice({ start, end }: LineRange): string {
        return this.text.slice(this.start(start), this.start(end));
    }

    /** Find the line an offset is on. */
    lineAt(offset: number): number {
        // halve the lines until one is left
        let low = 0;
        let high = this.starts.length - 1;
        while (low < high) {
            const middle = (low + high + 1) >>> 1;
            if (this.start(middle) <= offset) {
                low = middle;
            } else {
                high = middle - 1;
            }
        }

        return low;
    }

    /** Replace some text, or several parts of it at once, each by its offsets before. */
    edit(edits: Edit | readonly Edit[]): Source {
        // read a text with several replacements again
        const list = Edit.list(edits);
        const [only] = list;
        if (list.length !== 1 || only === undefined) {
            return list.length === 0 ? this : Source.of(this.#replaced(list));
        }

        // find the lines the inserted text has
        const { from, to, insert } = only;
        const text = this.#replaced(list);
        const inserted: number[] = [];
        for (let at = 0; at < insert.length; at += 1) {
            if (insert.charCodeAt(at) === LINE_BREAK) {
                inserted.push(from + at + 1);
            }
        }

        // keep the lines before, put in the new ones, and shift those after
        const head = this.lineAt(from) + 1;
        const tail = this.lineAt(to) + 1;
        const shift = insert.length - (to - from);
        const starts = new Uint32Array(head + inserted.length + (this.starts.length - tail));
        starts.set(this.starts.subarray(0, head));
        starts.set(inserted, head);
        const after = head + inserted.length;
        for (let line = tail; line < this.starts.length; line += 1) {
            starts[after + line - tail] = this.start(line) + shift;
        }

        return new Source(text, starts);
    }

    /** Write the text with replacements, refusing those outside it or over each other. */
    #replaced(edits: readonly Edit[]): string {
        // write what is between them as it is
        let written = "";
        let at = 0;
        for (const { from, to, insert } of edits) {
            if (from < at || to < from || to > this.text.length) {
                throw new RangeError("edit is invalid: it is outside the text or over another");
            }
            written += this.text.slice(at, from) + insert;
            at = to;
        }

        return written + this.text.slice(at);
    }
}
