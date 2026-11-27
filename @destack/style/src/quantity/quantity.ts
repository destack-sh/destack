/** A number and its unit as a person types them, such as `12`, `1.5rem` or `50%`. */
const NUMBER = /^(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?([a-z]+|%)?/iu;

/** The operators of an expression, by how tightly they bind. */
const PRECEDENCE = { "+": 1, "-": 1, "*": 2, "/": 2 } as const;

/** An operator of an expression. */
type Operator = keyof typeof PRECEDENCE;

/** A number and its unit, read from what a person types, such as `(8 + 4) * 2px`. */
export class Quantity {
    /** The number. */
    readonly value: number;
    /** The unit, such as `px` or `%`, none for a plain number. */
    readonly unit: string | undefined;

    /** Say a number in a unit. */
    constructor(value: number, unit: string | undefined) {
        this.value = value;
        this.unit = unit;
    }

    /** Read an expression of numbers, `+ - * /` and parentheses, whose units all agree. */
    static parse(text: string): Quantity | undefined {
        // split the text, then read one expression from all of it
        const tokens = tokenize(text);
        if (tokens === undefined) {
            return undefined;
        }
        const reader = new Reader(tokens);
        const quantity = reader.expression(0);

        return reader.isDone() && quantity !== undefined && Number.isFinite(quantity.value)
            ? quantity
            : undefined;
    }

    /** Combine two quantities by an operator, refusing two different units. */
    combine(operator: Operator, other: Quantity): Quantity | undefined {
        // take the one unit the two have, refusing two that differ
        if (this.unit !== undefined && other.unit !== undefined && this.unit !== other.unit) {
            return undefined;
        }
        const unit = this.unit ?? other.unit;

        // apply the operator to the numbers
        switch (operator) {
            case "+":
                return new Quantity(this.value + other.value, unit);
            case "-":
                return new Quantity(this.value - other.value, unit);
            case "*":
                return new Quantity(this.value * other.value, unit);
            case "/":
                return new Quantity(this.value / other.value, unit);
        }
    }
}

/** A token of an expression: a quantity, an operator or a parenthesis. */
type Token = Quantity | string;

/** A reader of an expression's tokens, by precedence climbing. */
class Reader {
    /** The tokens. */
    readonly #tokens: readonly Token[];
    /** The index of the next token. */
    #index = 0;

    /** Read tokens from the first. */
    constructor(tokens: readonly Token[]) {
        this.#tokens = tokens;
    }

    /** Tell whether every token was read. */
    isDone(): boolean {
        return this.#index === this.#tokens.length;
    }

    /** Read an expression whose operators bind at least as tightly as a precedence. */
    expression(least: number): Quantity | undefined {
        // read the first operand, then fold in each operator that binds tightly enough
        let left = this.#operand();
        let operator = this.#operator(least);
        while (left !== undefined && operator !== undefined) {
            this.#index += 1;
            const right = this.expression(PRECEDENCE[operator] + 1);
            left = right === undefined ? undefined : left.combine(operator, right);
            operator = this.#operator(least);
        }

        return left;
    }

    /** Read the next token as an operator that binds at least as tightly as a precedence. */
    #operator(least: number): Operator | undefined {
        const token = this.#tokens[this.#index];

        return typeof token === "string" && isOperator(token) && PRECEDENCE[token] >= least
            ? token
            : undefined;
    }

    /** Read a quantity, a signed operand or a parenthesized expression. */
    #operand(): Quantity | undefined {
        // take the next token, a quantity as it is
        const token = this.#tokens[this.#index];
        this.#index += 1;
        if (token instanceof Quantity) {
            return token;
        }
        // negate the operand after a minus
        else if (token === "-" || token === "+") {
            const operand = this.#operand();

            return operand === undefined || token === "+"
                ? operand
                : new Quantity(-operand.value, operand.unit);
        }
        // read the expression inside parentheses
        else if (token === "(") {
            const inner = this.expression(1);
            const closing = this.#tokens[this.#index];
            this.#index += 1;

            return closing === ")" ? inner : undefined;
        }

        return undefined;
    }
}

/** Split an expression into quantities, operators and parentheses, none for other text. */
function tokenize(text: string): Token[] | undefined {
    // read from the start, a token at a time
    const tokens: Token[] = [];
    let rest = text.trim();
    while (rest.length > 0) {
        // read a number and its unit, else one operator or parenthesis
        const number = NUMBER.exec(rest);
        const character = rest.charAt(0);

        // a number and its unit
        if (number !== null) {
            tokens.push(new Quantity(Number.parseFloat(number[0]), number[1]?.toLowerCase()));
            rest = rest.slice(number[0].length).trimStart();
        }
        // an operator or a parenthesis
        else if ("+-*/()".includes(character)) {
            tokens.push(character);
            rest = rest.slice(1).trimStart();
        }
        // other text, which no expression holds
        else {
            return undefined;
        }
    }

    return tokens;
}

/** Tell whether a token is an operator. */
function isOperator(token: string): token is Operator {
    return Object.hasOwn(PRECEDENCE, token);
}
