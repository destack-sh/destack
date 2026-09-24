/** The public portable representations of one content page. */
export type PageFormats = {
    /** The public Markdown source URL. */
    markdownRoute: string;

    /** The public plain-text source URL. */
    textRoute: string;
};

/** The portable representations and size of one content page. */
export type PageSource = PageFormats & {
    /** The approximate language-model token count. */
    tokens: number;
};
