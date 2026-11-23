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
            // Git submodules
            "test262/**",
            "babel/**",
            "eslint_typescript/**",
            // Test fixtures
            "tests/fixtures/data/**",
            "tests/estree/cases/**",
            "tests/estree/fixtures/**",
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
