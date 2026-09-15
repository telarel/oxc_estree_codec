function fn<T extends object = {}>(x: T): asserts x is T {
    throw new Error();
}
declare function g(this: Foo, a, ...rest): void;
const h = <T>(x: T): T => x;
