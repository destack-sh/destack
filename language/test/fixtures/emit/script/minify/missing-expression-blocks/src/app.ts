let r = 1;
let g;

g = () => {
    if (r) {
        undefined;
    }
};

g = () => {
    if (r) {
    } else if (r) {
        undefined;
    }
};

g = () => {
    if (r) {
        undefined;
    } else if (r) {
        undefined;
    }
};

g = () => {
    if (r) {
    } else if (r) {
    } else {
        undefined;
    }
};

g = () => {
    if (r) {
    } else if (r) {
        undefined;
    } else {
    }
};

g = () => {
    if (r) {
        undefined;
    } else if (r) {
    } else {
    }
};

g = () => {
    if (r) {
        undefined;
    } else if (r) {
        undefined;
    } else {
    }
};

g = () => {
    if (r) {
        undefined;
    } else if (r) {
        undefined;
    } else {
        undefined;
    }
};

g = () => {
    if (r) {
        undefined;
    } else if (r) {
    } else {
        undefined;
    }
};

g = () => {
    while (r) {
        undefined;
    }
};

g = () => {
    do undefined;
    while (r);
};

g = () => {
    for (;;) undefined;
};

g = () => {
    for (let i = 0; i < 10; i++) undefined;
};

g = () => {
    for (let i in [1, 2, 3]) undefined;
};

g = () => {
    for (let i of [1, 2, 3]) undefined;
};

g = () => {
    switch (r) {
        case 1:
            undefined;
        case 23: {
            undefined;
        }
    }
};

g = () => {
    let gg;
    gg = () => undefined;
};

console.log("PASS");
