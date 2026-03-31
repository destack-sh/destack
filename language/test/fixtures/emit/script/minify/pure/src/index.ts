let state = 0;

const unused = /*@__PURE__*/ (() => {
    state++;
    return state;
})();
const alsoUnused = /*@__PURE__*/ (() => {
    state += 10;
    return state;
})();

console.log(state);
