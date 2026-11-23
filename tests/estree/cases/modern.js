export class Account {
    static #instances = 0;

    balance = 0;
    #owner;

    static {
        Account.#instances = 0;
    }

    constructor(owner, balance = 1_000) {
        this.#owner = owner;
        this.balance = balance;
        Account.#instances += 1;
    }

    get owner() {
        return this.#owner;
    }

    deposit(amount) {
        this.balance ??= 0;
        this.balance += amount ?? 0;
        return this.balance;
    }
}

export function lookup(data, key) {
    return data?.items?.[key]?.value ?? "missing";
}

export const label = lookup({ items: { primary: { value: "ok" } } }, "primary");
