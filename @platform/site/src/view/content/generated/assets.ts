/** Current rendered bodies and documentation metadata. */
export const assets: Record<string, () => Promise<string>> = {
    "/_content/docs/index.json": () =>
        import("../../../../public/_content/docs/index.json?raw").then((module) => module.default),
    "/_content/docs/setup/index.json": () =>
        import("../../../../public/_content/docs/setup/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/tree/7428fdebe0132c8baa0a7603e425d77c0d0bf10ff73b53f35d74f31c0d392972.json": () =>
        import("../../../../public/_content/tree/7428fdebe0132c8baa0a7603e425d77c0d0bf10ff73b53f35d74f31c0d392972.json?raw").then(
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
