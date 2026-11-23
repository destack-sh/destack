import * as style from "@destack/style";
import { size } from "@destack/theme/tokens.stylex";
import { type Accessor, createSignal, type JSX } from "@destack/view";
import { createElementSize } from "@destack/view/primitives/resize-observer";
import { Length } from "../virtualizer/index.ts";

/** The styles of a picker as wide as its columns, or as the room it has. */
const styles = style.create({
    wide: (count: number) => ({
        width: `calc(${String(count)} * ${size[2]})`,
        maxWidth: "100%",
    }),
});

/** The fit of a picker's square cells in its width: the columns it asks for, or as many as fit. */
export class ColumnFit {
    /** The columns to draw. */
    readonly columns: Accessor<number>;
    /** Take the picker's element, whose width the columns fit. */
    readonly measure: (element: HTMLElement) => void;
    /** The columns the picker asks for. */
    readonly #asked: Accessor<number>;
    /** The width of a cell. */
    readonly #cell: Length;

    /** Fit the columns a picker asks for into its width. */
    constructor(asked: Accessor<number>) {
        // measure the picker and a cell
        const [element, setElement] = createSignal<HTMLElement | undefined>(undefined, {
            ownedWrite: true,
        });
        const measured = createElementSize(element);
        this.#asked = asked;
        this.#cell = new Length(() => size[2]);
        this.measure = (created) => {
            setElement(created);
        };

        // draw the ones asked for until both are measured
        this.columns = () => {
            const cell = this.#cell.pixels();
            const { width } = measured;

            return cell === undefined || cell === 0 || width === null || width === 0
                ? asked()
                : Math.max(1, Math.min(asked(), Math.floor((width + 0.5) / cell)));
        };
    }

    /** Read the style of a picker as wide as its columns. */
    style(): style.Styles {
        return styles.wide(this.#asked());
    }

    /** Render the cell whose width is measured. */
    render(): JSX.Element {
        return this.#cell.render();
    }
}
