import { Language } from "@destack/code";

/** What a file is, as far as its icon tells. */
export type FileKind =
    | "archive"
    | "audio"
    | "code"
    | "image"
    | "other"
    | "pdf"
    | "sheet"
    | "text"
    | "video";

/** The kinds of the extensions that media types leave open, or that come without one. */
const EXTENSIONS: ReadonlyMap<string, FileKind> = new Map<string, FileKind>([
    // archives
    ["7z", "archive"],
    ["gz", "archive"],
    ["rar", "archive"],
    ["tar", "archive"],
    ["zip", "archive"],

    // sheets
    ["csv", "sheet"],
    ["ods", "sheet"],
    ["tsv", "sheet"],
    ["xls", "sheet"],
    ["xlsx", "sheet"],

    // text
    ["doc", "text"],
    ["docx", "text"],
    ["md", "text"],
    ["odt", "text"],
    ["rtf", "text"],
    ["txt", "text"],

    // the rest
    ["pdf", "pdf"],
]);

/** The kinds of files. */
export const FileKind = {
    /** Read a file's kind by its name and its media type, if it has one. */
    of(name: string, type = ""): FileKind {
        // take the extension's kind, a picture, code, which browsers type as video, then the media
        const extension = name.slice(name.lastIndexOf(".") + 1).toLowerCase();
        const [family] = type.split("/", 1);
        const known = EXTENSIONS.get(extension);
        if (known !== undefined) {
            return known;
        } else if (family === "image") {
            return "image";
        } else if (Language.ofFile(name) !== undefined) {
            return "code";
        } else if (family === "audio" || family === "video" || family === "text") {
            return family;
        }

        return type === "application/pdf" ? "pdf" : "other";
    },
};
