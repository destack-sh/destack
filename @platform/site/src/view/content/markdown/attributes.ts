/** Read one capture group that a successful match always fills. */
export function captured(match: RegExpMatchArray | RegExpExecArray, group: number) {
    const value = match[group];
    if (value == undefined) {
        throw new Error(`capture group '${group}' in '${match[0]}' not found`);
    }

    return value;
}

/** Parse the quoted and unquoted attributes of a fence or an image. */
export function parseAttributes(source: string) {
    // read each key and its quoted or bare value
    const attributes: Record<string, string> = {};
    for (const match of source.matchAll(/(\w+)=(?:"([^"]*)"|'([^']*)'|(\S+))/gu)) {
        const value = match[2] ?? match[3] ?? match[4];
        if (value == undefined) {
            throw new Error(`attribute value in '${match[0]}' not found`);
        }
        attributes[captured(match, 1)] = value;
    }

    // read a bare word as the kind
    if (!source.includes("=") && source.trim() !== "") {
        attributes["kind"] = source.trim();
    }

    return attributes;
}
