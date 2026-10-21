/** Current rendered bodies and documentation metadata. */
export const assets: Record<string, () => Promise<string>> = {
    "/_content/docs/index.json": () =>
        import("../../../../public/_content/docs/index.json?raw").then((module) => module.default),
    "/_content/docs/setup/index.json": () =>
        import("../../../../public/_content/docs/setup/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/tree/036238cd7c5057c41036dcabd705302ba5d26aa6d976b6ac020d403982fb9112.json": () =>
        import("../../../../public/_content/tree/036238cd7c5057c41036dcabd705302ba5d26aa6d976b6ac020d403982fb9112.json?raw").then(
            (module) => module.default,
        ),
    "/_content/tree/9bbcb3be4bf31fcf22bcacc3dedd279e5dd88b5365729fbe7f005c52d407d4c7.json": () =>
        import("../../../../public/_content/tree/9bbcb3be4bf31fcf22bcacc3dedd279e5dd88b5365729fbe7f005c52d407d4c7.json?raw").then(
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
