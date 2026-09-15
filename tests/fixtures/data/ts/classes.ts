abstract class A implements I {
    #x = 1;
    static s: number = 2;
    declare d?: string;
    get g(): number {
        return this.#x;
    }
    static {
        init();
    }
}
class B extends A<string> {
    m<T>(x: T): asserts x is T {}
}
