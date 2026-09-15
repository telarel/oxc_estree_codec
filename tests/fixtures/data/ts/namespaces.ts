namespace NS {
    export const x = 1;
}
declare global {
    interface Window {
        oxc: boolean;
    }
}
declare module "mod" {
    export const y: number;
}
declare namespace G {
    const v: string;
}
