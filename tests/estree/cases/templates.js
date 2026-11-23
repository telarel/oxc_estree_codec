const empty = ``;
const plain = `foo`;
const substitution = `${a}`;
const suffix = `${a}b`;
const prefix = `a${b}`;
const mixed = `a${b}c${d}e`;
const escaped = `line\nbreak\ttab\`quote\${not a substitution}`;
const unicode = `café 日本語 😀`;

function tag(strings, ...values) {
    return strings.raw.join("|") + values.length;
}

const tagged = tag`tagged ${plain} value`;
const taggedEscaped = tag`escaped\nraw`;
