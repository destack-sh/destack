export type ViewerFile = {
    html: string;
    kind: "markdown" | "output" | "source" | "terminal";
    name: string;
    path: string;
    source: string;
};

export type Status = {
    cursor: Cursor;
    file: string;
    files: number;
    lines: number;
    mode: ViewerFile["kind"];
};

export type Cursor = {
    column: number;
    line: number;
};

export type MarkdownLinkHandler = (href: string, basePath: string) => boolean;
