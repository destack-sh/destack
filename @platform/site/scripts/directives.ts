/// Transform block directives while preserving code and HTML comments.
export function mapDirectives(markdown: string, render: (name: string, attributes: Record<string, string>, body: string) => string) {
    const lines = markdown.split("\n");
    const output = [];
    let fence: string | undefined;
    let comment = false;

    for (let index = 0; index < lines.length; index += 1) {
        const line = lines[index];
        const delimiter = /^ {0,3}(`{3,}|~{3,})(.*)$/.exec(line);

        // leave fenced listings and comments unchanged
        if (fence != undefined) {
            if (
                delimiter &&
                delimiter[1][0] === fence[0] &&
                delimiter[1].length >= fence.length &&
                delimiter[2].trim() === ""
            ) {
                fence = undefined;
            }
            output.push(line);
            continue;
        }
        if (comment || line.trimStart().startsWith("<!--")) {
            comment = !line.includes("-->");
            output.push(line);
            continue;
        }
        if (delimiter) {
            fence = delimiter[1];
            output.push(line);
            continue;
        }

        // render one complete directive
        const directive = line.startsWith(":::")
            ? parseDirective(lines.slice(index).join("\n"))
            : undefined;
        if (directive) {
            output.push(render(directive.name, directive.attributes, directive.body));
            index += directive.raw.split("\n").length - 1;
        } else {
            output.push(line);
        }
    }

    return output.join("\n");
}

/// Parse a directive at the beginning of a Markdown block.
export function parseDirective(source: string) {
    const opening = /^:::(\w+)(?:[ \t]+([^\n]*))?(?:\n|$)/.exec(source);
    if (opening == null) {
        return;
    }

    // accept single-line directives and preserve fenced code in multiline bodies
    const attributes = opening[2] ?? "";
    if (attributes.endsWith(":::")) {
        return {
            raw: opening[0].trimEnd(),
            name: opening[1],
            attributes: parseAttributes(attributes.slice(0, -3)),
            body: "",
        };
    }

    let length = opening[0].length;
    let fence: string | undefined;
    for (const line of source.slice(length).split("\n")) {
        const delimiter = /^ {0,3}(`{3,}|~{3,})(.*)$/.exec(line);
        if (fence != undefined) {
            if (
                delimiter &&
                delimiter[1][0] === fence[0] &&
                delimiter[1].length >= fence.length &&
                delimiter[2].trim() === ""
            ) {
                fence = undefined;
            }
        } else if (delimiter) {
            fence = delimiter[1];
        } else if (line === ":::") {
            return {
                raw: source.slice(0, length + line.length),
                name: opening[1],
                attributes: parseAttributes(attributes),
                body: source.slice(opening[0].length, length).trimEnd(),
            };
        }
        length += line.length + 1;
    }

    throw new Error(`unclosed ${opening[1]} directive`);
}

/// Parse quoted and unquoted directive or fence attributes.
export function parseAttributes(source: string) {
    const attributes: Record<string, string> = {};
    for (const match of source.matchAll(/(\w+)=(?:"([^"]*)"|'([^']*)'|(\S+))/g)) {
        attributes[match[1]] = match[2] ?? match[3] ?? match[4];
    }

    if (!source.includes("=") && source.trim() !== "") {
        attributes.kind = source.trim();
    }

    return attributes;
}
