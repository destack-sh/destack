/** The breadboard's width in hole cells. */
export const boardCells = 88;
/** The hole cells left clear between the board edge and the outer cards. */
export const boardInset = 3;
/** The hole rows in each figure row. */
export const rowCells = 9;
/** The width of each column in a row of three cards, in cells. */
export const columnWidth = 24;
/** The left edge of each column in a row of three cards, in cells: five-cell gaps between them, three-cell insets at the edges. */
export const columnLefts = [3, 32, 61];
/** The centre of each column in a row of three cards, in cells. */
export const columnCentres = columnLefts.map((left) => left + columnWidth / 2);
/** The gap between the cards of a row of four, in cells. */
export const quarterGap = 3;
/** The width of each card in a row of four, in cells. */
export const quarterWidth = (boardCells - boardInset * 2 - quarterGap * 3) / 4;
/** The left edge of each card in a row of four, in cells. */
export const quarterLefts = [0, 1, 2, 3].map(
    (index) => boardInset + index * (quarterWidth + quarterGap),
);
/** The centre of each card in a row of four, in cells. */
export const quarterCentres = quarterLefts.map((left) => left + quarterWidth / 2);
