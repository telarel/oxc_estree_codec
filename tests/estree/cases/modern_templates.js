const name = "world";
const nested = `outer ${`inner ${name}`} tail`;

function passthrough(strings, ...values) {
    return strings.raw.join("") + values.length;
}

const invalidEscape = passthrough`\unicode and \xZZ`;
