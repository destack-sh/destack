export const mergedVarSnapshots = (() => {
    var a = 1;
    var b = 2;
    var c = 3;

    a = 4;
    const first = [a, b, c];
    b = 5;
    const second = [a, b, c];
    c = 6;
    const third = [a, b, c];

    return [first, second, third];
})();
