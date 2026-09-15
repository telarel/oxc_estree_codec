export interface User {
    readonly id: bigint;
    name: string;
    email?: string | null;
    roles: Array<"admin" | "editor" | "viewer">;
    metadata?: Record<string, unknown>;
}
interface Ext extends A<number>, B {
    readonly [k: string]: unknown;
    m(x: string): void;
}
