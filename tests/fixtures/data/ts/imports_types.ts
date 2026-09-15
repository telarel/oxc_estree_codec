type A = import("mod").Thing;
type B = import("mod").Nested.Deep<string>;
const c: A.B.C = null as never;
