export const padZero = function(num) {
    return String(num).padStart(2, "0");
};

export const formatDate = function(date) {
    return `${date.getFullYear()}-${padZero(date.getMonth() + 1)}-${padZero(date.getDate())}`;
};

export const greeting = function(name) {
    return `Hello, ${name}!`;
};

export function renderSummary(name) {
    return `${greeting(name)}:${padZero(7)}`;
}

console.log(greeting("World"));
console.log(formatDate(new Date()));
console.log(renderSummary("World"));
//# sourceMappingURL=./site.js.map
