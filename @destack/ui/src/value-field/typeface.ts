import { schema } from "@destack/schema";
import { Typeface } from "@destack/style";
import { type Accessor, createContext, createSignal, type Setter, useContext } from "@destack/view";

/** The fonts the Local Font Access API lists, as far as a catalog reads them. */
const LOCAL_FONTS = schema.array(
    schema.looseObject({ family: schema.string(), style: schema.string() }),
);

/** The typefaces a picker offers: the document's, the device's once asked, CSS's generic ones. */
export class TypefaceCatalog {
    /** The device's typefaces, none until a person asks for them. */
    readonly local: Accessor<readonly Typeface[]>;
    /** Replace the device's typefaces. */
    readonly #setLocal: Setter<readonly Typeface[]>;
    /** The document whose faces it reads. */
    readonly #document: Document;

    /** Create the catalog of a document, without the device's fonts until asked. */
    constructor(document: Document) {
        this.#document = document;
        [this.local, this.#setLocal] = createSignal<readonly Typeface[]>([], { ownedWrite: true });
    }

    /** Tell whether the browser lists the device's fonts, as the Local Font Access API does. */
    get isLocalListed(): boolean {
        return (
            typeof Reflect.get(this.#document.defaultView ?? {}, "queryLocalFonts") === "function"
        );
    }

    /** Read the typefaces the document's `@font-face` rules and loaded fonts declare. */
    document(): Typeface[] {
        return Typeface.document(this.#document.fonts);
    }

    /** List CSS's generic families. */
    generic(): Typeface[] {
        return Typeface.generic();
    }

    /** Find the typeface of a family among the document's, the device's and the generic ones. */
    find(family: string): Typeface | undefined {
        return [...this.document(), ...this.local(), ...this.generic()].find(
            (typeface) => typeface.family === family,
        );
    }

    /** Ask the browser for the device's fonts, which it lets a person allow or refuse. */
    async listLocal(): Promise<void> {
        // call the API on the window, and read what it gives as any foreign input
        const window = this.#document.defaultView;
        const query: unknown = window === null ? undefined : Reflect.get(window, "queryLocalFonts");
        if (typeof query !== "function") {
            throw new TypeError("the browser lists no local fonts");
        }
        const fonts = LOCAL_FONTS.parse(await Reflect.apply(query, window, []));
        this.#setLocal(Typeface.local(fonts));
    }
}

/** The typeface of the family the value around sets, for a weight that lists its faces. */
export const TypefaceContext = createContext<Accessor<Typeface | undefined>>(() => undefined);

/** Read the typeface of the family the value around sets, none outside one. */
export function useTypeface(): Accessor<Typeface | undefined> {
    return useContext(TypefaceContext);
}
