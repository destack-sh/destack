/** The breadboard's width in hole cells. */
export const boardCells = 88;
/** The hole cells left clear between the board edge and the outer cards. */
export const boardInset = 6;
/** The hole rows in each figure row. */
export const rowCells = 9;
/** The width of each column in a row of three cards, in cells. */
export const columnWidth = 22;
/** The left edge of each column in a row of three cards, in cells: five-cell gaps between them. */
export const columnLefts = [6, 33, 60];
/** The centre of each column in a row of three cards, in cells. */
export const columnCentres = columnLefts.map((left) => left + columnWidth / 2);
