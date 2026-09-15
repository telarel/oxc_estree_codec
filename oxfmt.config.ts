import { defineConfig } from "@apst/oxfmt";
import { IGNORE_PATTERNS_DEFAULT } from "@apst/oxfmt/constants/ignore-patterns";

export default defineConfig({
    ignorePatterns: [
        ...IGNORE_PATTERNS_DEFAULT,
        // Rust
        "target/**",
        // Git submodules
        "test262/**",
        "babel/**",
        "eslint_typescript/**",
    ],
});
