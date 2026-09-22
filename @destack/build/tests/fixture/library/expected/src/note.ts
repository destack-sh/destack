/** An editable note. */
export interface Note {
    /** The note title. */
    title: string;
    /** Whether the note is complete. */
    complete: boolean;
}

/** Create an incomplete note. */
export function createNote(title: string): Note {
    return { title, complete: false };
}
