export const mergedVarSnapshots = (() => {
    let a = 1, b = 2, c = 3;
    a=4;
    const first = [a, b, c];
    b=5;
    const second = [a, b, c];
    c=6;
    const third = [a, b, c];
    return [first, second, third];
})();
