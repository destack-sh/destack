/** Build the template-string sample cluster used by the minify fixture. */
export function buildTemplateSamples() {
    return [
        `${1}-${2}-${3}-${null}-${undefined}-${true}-${false}`,
        `😋📋👌`.length === 6,
        `😋📋👌`.length == 6,
        `😋📋👌`.length === 2,
        `😋📋👌`.length == 2,
    ];
}
