/** Current rendered bodies and documentation metadata. */
export const assets: Record<string, () => Promise<string>> = {
    "/_content/docs/index.json": () =>
        import("../../../../public/_content/docs/index.json?raw").then((module) => module.default),
    "/_content/docs/setup/index.json": () =>
        import("../../../../public/_content/docs/setup/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/tree/4313e1b3bae0ab6ca0cd47516f6d2b46640ed06f798fbe7888f6c73573f13036.json": () =>
        import("../../../../public/_content/tree/4313e1b3bae0ab6ca0cd47516f6d2b46640ed06f798fbe7888f6c73573f13036.json?raw").then(
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
};
