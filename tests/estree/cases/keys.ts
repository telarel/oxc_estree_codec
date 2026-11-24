const key = "dynamic";

interface Shape {
    plain: number;
    "string": number;
    3: number;
    [key]: number;
    method(): void;
    "quoted method"(): void;
    4(): void;
    [key](): void;
}

class Typed {
    plain: number = 1;
    "string": number = 2;
    3: number = 3;
    [key]: number = 4;
    method(): void {}
    "quoted method"(): void {}
    4(): void {}
    [key](): void {}
    readonly readonlyProperty: number = 5;
    private privateMethod(): void {}
    static staticMethod(): void {}
    static [key](): void {}
}

export { Typed };
