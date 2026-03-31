function x() {
    return 1;
}
const first = x();
function x() {
    return 2;
}
const second = x();
function x() {
    return 3;
}
export const functionThreeSamples = [first, second, x()];
