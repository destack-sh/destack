import { LlmsOptions } from "./site.ts";

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
