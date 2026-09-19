import { defineConfig } from "@apst/oxlint";
import { IGNORE_PATTERNS_DEFAULT } from "@apst/oxlint/constants/ignore-patterns";
import { commonPreset } from "@apst/oxlint/presets/common";
import { jsxPreset } from "@apst/oxlint/presets/jsx";
import { nodePreset } from "@apst/oxlint/presets/node";
import { reactPreset } from "@apst/oxlint/presets/react";

export default defineConfig(
    {
        ignorePatterns: [
            ...IGNORE_PATTERNS_DEFAULT,
            // Rust
            "target/**",
            // Test fixtures
            "tests/fixtures/data/**",
            // Git submodules
            "test262/**",
            "babel/**",
            "eslint_typescript/**",
        ],
        options: {
            typeAware: true,
            typeCheck: true,
        },
    },
    [
        // Foundation
        commonPreset(),
        // Environment
        nodePreset(),
        // Framework
        jsxPreset(),
        reactPreset(),
    ],
);
