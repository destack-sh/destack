/** Current rendered bodies and documentation metadata. */
export const assets: Record<string, () => Promise<string>> = {
    "/_content/docs/index.json": () =>
        import("../../../../public/_content/docs/index.json?raw").then((module) => module.default),
    "/_content/docs/setup/index.json": () =>
        import("../../../../public/_content/docs/setup/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/tree/1821d251cf091e2b1edcad5e2e9d5d666a1fcf88b2cbb3b21c8666c734840145.json": () =>
        import("../../../../public/_content/tree/1821d251cf091e2b1edcad5e2e9d5d666a1fcf88b2cbb3b21c8666c734840145.json?raw").then(
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
    "/_content/tree/d1d4b8a9d8548e19153001b7086fdbdf22a7dbc533708ab95bd0985490418411.json": () =>
        import("../../../../public/_content/tree/d1d4b8a9d8548e19153001b7086fdbdf22a7dbc533708ab95bd0985490418411.json?raw").then(
            (module) => module.default,
        ),
};
