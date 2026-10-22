/** Current rendered bodies and documentation metadata. */
export const assets: Record<string, () => Promise<string>> = {
    "/_content/docs/index.json": () =>
        import("../../../../public/_content/docs/index.json?raw").then((module) => module.default),
    "/_content/docs/setup/index.json": () =>
        import("../../../../public/_content/docs/setup/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/tree/c0f66583508180752f49d9a538c84c5bab263b9e17e567f30fbc12aa4c495d4d.json": () =>
        import("../../../../public/_content/tree/c0f66583508180752f49d9a538c84c5bab263b9e17e567f30fbc12aa4c495d4d.json?raw").then(
            (module) => module.default,
        ),
    "/_content/tree/c421eadfbad5ee67f1f382200fe1946c0c8ccb91f7069f140b1ceffefb9e239e.json": () =>
        import("../../../../public/_content/tree/c421eadfbad5ee67f1f382200fe1946c0c8ccb91f7069f140b1ceffefb9e239e.json?raw").then(
            (module) => module.default,
        ),
    "/_content/tree/cbcaec514612bdddf94c3c424c5e9e82f09e9defc28f68f4732170204c483257.json": () =>
        import("../../../../public/_content/tree/cbcaec514612bdddf94c3c424c5e9e82f09e9defc28f68f4732170204c483257.json?raw").then(
            (module) => module.default,
        ),
    "/_content/tree/d022fd6f324a46fff2d37cc263be815f8a6b8b6951ffff2feddfb354ca0baa6a.json": () =>
        import("../../../../public/_content/tree/d022fd6f324a46fff2d37cc263be815f8a6b8b6951ffff2feddfb354ca0baa6a.json?raw").then(
            (module) => module.default,
        ),
};
