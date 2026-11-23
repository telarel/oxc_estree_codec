export interface User {
    readonly id: number;
    name?: string;
    tags: readonly string[];
}

export type UserId = User["id"] | string;

export enum Role {
    Admin = "admin",
    Member = "member",
}

export namespace Validation {
    export function isUserId(value: string): boolean {
        return value.length > 0;
    }
}

export function first<T>(items: readonly T[]): T | undefined {
    return items[0];
}

export class Account {
    constructor(
        public readonly id: UserId,
        private balance: number = 0,
    ) {}

    get amount(): number {
        return this.balance as number;
    }
}

const raw = { id: 1, name: "Ada" };
const user = raw as User;
const checked = { id: 2, name: "Grace", tags: [] } satisfies User;
