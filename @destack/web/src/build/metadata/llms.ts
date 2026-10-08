import { schema } from "@destack/schema";

/** A site's guide for language models, in the llms.txt format (llmstxt.org). */
export const LlmsOptions = schema.object({
    /** The site's name. */
    title: schema.string().min(1),
    /** The one-line summary models read first. */
    summary: schema.string().min(1),
    /** The paragraphs about the site below the summary. */
    details: schema.string().exactOptional(),
    /** The sections of links, each to a page's Markdown. */
    sections: schema
        .array(
            schema.object({
                /** The section's title, `Optional` for links a model may skip. */
                title: schema.string().min(1),
                /** The section's links. */
                links: schema
                    .array(
                        schema.object({
                            /** The page's title. */
                            title: schema.string().min(1),
                            /** The path of the page's Markdown. */
                            path: schema.string().startsWith("/"),
                            /** What the page holds. */
                            description: schema.string().exactOptional(),
                        }),
                    )
                    .readonly(),
            }),
        )
        .readonly(),
});
/** A site's guide for language models. */
export type LlmsOptions = schema.Infer<typeof LlmsOptions>;

/** Write a site's llms.txt: its title, summary and details, then its sections of links. */
export function writeLlms(origin: string, options: LlmsOptions): string {
    // write each section as a heading over its links
    const sections = options.sections.map((section) =>
        [
            `## ${section.title}`,
            "",
            ...section.links.map((link) => {
                const address = new URL(link.path, origin).href;
                const description = link.description === undefined ? "" : `: ${link.description}`;

                return `- [${link.title}](${address})${description}`;
            }),
        ].join("\n"),
    );

    return `${[
        `# ${options.title}`,
        `> ${options.summary}`,
        ...(options.details === undefined ? [] : [options.details]),
        ...sections,
    ].join("\n\n")}\n`;
}
