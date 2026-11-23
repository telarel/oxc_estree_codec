"use strict";

import defaultExport from "basic-default";
import { named, renamed as alias } from "basic-named";
import * as namespace from "basic-namespace";

export const answer = 42;

export function add(left, right = 1, ...rest) {
    return left + right + rest.length;
}

export default class Counter {
    constructor(start = 0) {
        this.value = start;
    }

    increment(step = 1) {
        this.value += step;
        return this.value;
    }
}

export { alias as reexported };

const object = { key: "value", nested: { list: [1, 2, 3] } };
const array = [0, ...object.nested.list, 4];
const double = (value) => value * 2;

function controlFlow(input) {
    if (input > 0) {
        return "positive";
    } else if (input < 0) {
        return "negative";
    }

    for (let index = 0; index < array.length; index += 1) {
        if (array[index] === input) {
            break;
        }
    }

    for (const item of array) {
        if (item === input) {
            continue;
        }
    }

    while (input < 10) {
        input += 1;
    }

    try {
        throw new Error(String(input));
    } catch (error) {
        input = error.message.length;
    } finally {
        input += 1;
    }

    switch (input) {
        case 0:
            return "zero";
        case 1:
            return "one";
        default:
            return `${namespace} ${double(input)} ${defaultExport}`;
    }
}

controlFlow(answer);
