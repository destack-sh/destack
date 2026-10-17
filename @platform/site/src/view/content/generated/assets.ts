/** Current rendered bodies and documentation metadata. */
export const assets: Record<string, () => Promise<string>> = {
    "/_content/docs/index.json": () =>
        import("../../../../public/_content/docs/index.json?raw").then((module) => module.default),
    "/_content/docs/setup/index.json": () =>
        import("../../../../public/_content/docs/setup/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/tree/26997ae9f9c39725a00acc61b52e397eaa575b18c208d2b1149eebce75323ffe.json": () =>
        import("../../../../public/_content/tree/26997ae9f9c39725a00acc61b52e397eaa575b18c208d2b1149eebce75323ffe.json?raw").then(
            (module) => module.default,
        ),
    "/_content/tree/66fff74db5de4760838bab6f7b14ee012851722408049856c2e2ddd7fb4ebd57.json": () =>
        import("../../../../public/_content/tree/66fff74db5de4760838bab6f7b14ee012851722408049856c2e2ddd7fb4ebd57.json?raw").then(
            (module) => module.default,
        ),
    "/_content/tree/c0f66583508180752f49d9a538c84c5bab263b9e17e567f30fbc12aa4c495d4d.json": () =>
        import("../../../../public/_content/tree/c0f66583508180752f49d9a538c84c5bab263b9e17e567f30fbc12aa4c495d4d.json?raw").then(
            (module) => module.default,
        ),
    "/_content/tree/cbcaec514612bdddf94c3c424c5e9e82f09e9defc28f68f4732170204c483257.json": () =>
        import("../../../../public/_content/tree/cbcaec514612bdddf94c3c424c5e9e82f09e9defc28f68f4732170204c483257.json?raw").then(
            (module) => module.default,
        ),
};
