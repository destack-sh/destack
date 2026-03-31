function x() {
    return 1;
}
const first = x();
function x() {
    return 2;
}
export const functionTwoSamples = [first, x()];
