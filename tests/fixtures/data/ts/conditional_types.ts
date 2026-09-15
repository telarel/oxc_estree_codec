type B<T> = T extends infer U ? U : never;
type Distribute<T> = T extends Array<infer I> ? I : T;
type Wrap<T> = T extends string ? `${T}!` : never;
