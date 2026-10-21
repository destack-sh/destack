import { Derivation, type Deriver } from "@destack/identity";

/** The bytes of an installation root: a 256-bit secret. */
const ROOT_BYTES = 32;

/** The label an installation's root derives under from its space's identity. */
const INSTALLATION_ROOT_LABEL = "@destack/object/installation-root";

/** The root an installation's secrets and keys derive from, rooted in its space's identity. */
export const InstallationRoot = {
    /** Derive an installation's root from its space's identity, as its host does. */
    derive(space: Pick<Deriver, "derive">, installation: string): Promise<Uint8Array<ArrayBuffer>> {
        return space.derive({ label: INSTALLATION_ROOT_LABEL, context: installation });
    },

    /** Open an installation's root as the deriver of its keys. */
    async open(root: Uint8Array<ArrayBuffer>): Promise<Deriver> {
        if (root.length !== ROOT_BYTES) {
            throw new TypeError(`an installation root has ${ROOT_BYTES} bytes, not ${root.length}`);
        }

        return Derivation.of(await Derivation.root(root));
    },
};
