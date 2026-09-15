type M<T> = { [K in keyof T as `get${K & string}`]?: T[K] };
type R<T> = { [K in keyof T]-?: T[K] };
type Partial<T> = { [K in keyof T]+?: T[K] };
