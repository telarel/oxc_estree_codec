import "side";

import def, { named as alias, type T1 } from "mod" with { type: "json" };
import * as ns from "n";
import source ph from "p";
export const e = 1;
export { e as renamed };
export { q } from "r" with { k: "v" };
export * from "x";
export * as ns2 from "s";
export default e;
export type { T1 };
