const u: unique symbol = Symbol();
type Q = keyof typeof obj;
type RO = readonly string[];
let x: string | null = null;
x ??= "d";
const y = x!.length;
