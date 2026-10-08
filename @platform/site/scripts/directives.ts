/** Transform block directives while preserving code and HTML comments. */
export function mapDirectives(
    markdown: string,
    render: (name: string, attributes: Record<string, string>, body: string) => string,
) {
    // track the open fence and comment while copying lines
    const lines = markdown.split("\n");
    const output = [];
    let fence: string | undefined;
    let isComment = false;
    for (let index = 0; index < lines.length; index += 1) {
        const line = lines[index];
        if (line == undefined) {
            throw new Error(`missing Markdown line ${index}`);
        }
        const delimiter = /^ {0,3}(`{3,}|~{3,})(.*)$/u.exec(line);

        // leave fenced listings and comments unchanged
        if (fence != undefined) {
            if (delimiter && closesFence(delimiter, fence)) {
                fence = undefined;
            }
            output.push(line);
            continue;
        }
        if (isComment || line.trimStart().startsWith("<!--")) {
            isComment = !line.includes("-->");
            output.push(line);
            continue;
        }
        if (delimiter) {
            fence = captured(delimiter, 1);
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

/** Parse a directive at the beginning of a Markdown block. */
export function parseDirective(source: string) {
    // match the opening line
    const opening = /^:::(\w+)(?:[ \t]+([^\n]*))?(?:\n|$)/u.exec(source);
    if (opening == null) {
        return;
    }

    // accept single-line directives and preserve fenced code in multiline bodies
    const attributes = opening[2] ?? "";
    if (attributes.endsWith(":::")) {
        return {
            raw: opening[0].trimEnd(),
            name: captured(opening, 1),
            attributes: parseAttributes(attributes.slice(0, -3)),
            body: "",
        };
    }

    // close the directive at the first bare marker outside a fence
    let length = opening[0].length;
    let fence: string | undefined;
    for (const line of source.slice(length).split("\n")) {
        const delimiter = /^ {0,3}(`{3,}|~{3,})(.*)$/u.exec(line);
        if (fence != undefined) {
            if (delimiter && closesFence(delimiter, fence)) {
                fence = undefined;
            }
        } else if (delimiter) {
            fence = captured(delimiter, 1);
        } else if (line === ":::") {
            return {
                raw: source.slice(0, length + line.length),
                name: captured(opening, 1),
                attributes: parseAttributes(attributes),
                body: source.slice(opening[0].length, length).trimEnd(),
            };
        }
        length += line.length + 1;
    }

    // refuse a directive without its closing marker
    throw new Error(`unclosed ${opening[1]} directive`);
}

/** Return whether a fence delimiter closes the open fence. */
function closesFence(delimiter: RegExpExecArray, fence: string) {
    const marker = captured(delimiter, 1);

    return (
        marker[0] === fence[0] &&
        marker.length >= fence.length &&
        captured(delimiter, 2).trim() === ""
    );
}

/** Read one capture group that a successful match always fills. */
export function captured(match: RegExpMatchArray | RegExpExecArray, group: number) {
    const value = match[group];
    if (value == undefined) {
        throw new Error(`missing capture group ${group} in ${match[0]}`);
    }

    return value;
}

/** Parse quoted and unquoted directive or fence attributes. */
export function parseAttributes(source: string) {
    // read each key and its quoted or bare value
    const attributes: Record<string, string> = {};
    for (const match of source.matchAll(/(\w+)=(?:"([^"]*)"|'([^']*)'|(\S+))/gu)) {
        const value = match[2] ?? match[3] ?? match[4];
        if (value == undefined) {
            throw new Error(`missing attribute value in ${match[0]}`);
        }
        attributes[captured(match, 1)] = value;
    }

    // read a bare word as the kind
    if (!source.includes("=") && source.trim() !== "") {
        attributes["kind"] = source.trim();
    }

    return attributes;
}
