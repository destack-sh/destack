export const dep = { x: 42 };

export function log(value) {
    if(dep) {
        return console.log(value);
    }
}

log(dep);
