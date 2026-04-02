export const padZero = (num) => String(num).padStart(2, "0");

export const formatDate = (
    date,
) => `${date.getFullYear()}-${padZero(date.getMonth() + 1)}-${padZero(date.getDate())}`;

export const greeting = (name) => `Hello, ${name}!`;

export function renderSummary(name) {
    return `${greeting(name)}:${padZero(7)}`;
}

console.log(greeting("World"));
console.log(formatDate(new Date()));
console.log(renderSummary("World"));
//# sourceMappingURL=./site.js.map
