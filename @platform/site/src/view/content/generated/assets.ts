/** Current rendered bodies and documentation metadata. */
export const assets: Record<string, () => Promise<string>> = {
    "/_content/docs/index.json": () =>
        import("../../../../public/_content/docs/index.json?raw").then((module) => module.default),
    "/_content/docs/setup/index.json": () =>
        import("../../../../public/_content/docs/setup/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/tree/54567ed5c49cb3f310b98a83d1d30842aa776e9d3e427677680c20da7927b7e3.json": () =>
        import("../../../../public/_content/tree/54567ed5c49cb3f310b98a83d1d30842aa776e9d3e427677680c20da7927b7e3.json?raw").then(
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
    "/_content/tree/e28dec7ff2d8b4493b8008f14ec2db0faad84afd238c2f7634d6f507e6734976.json": () =>
        import("../../../../public/_content/tree/e28dec7ff2d8b4493b8008f14ec2db0faad84afd238c2f7634d6f507e6734976.json?raw").then(
            (module) => module.default,
        ),
};
