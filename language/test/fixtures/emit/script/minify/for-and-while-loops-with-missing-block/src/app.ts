{
    let n = 0;
    for (let i = 0; i < 10; i++) i;
}
{
    let j = 0;
    for (let i in [1, 2, 3]) i;
}
{
    let k = 0;
    for (let i of [1, 2, 3]) i;
}

console.log("PASS");
