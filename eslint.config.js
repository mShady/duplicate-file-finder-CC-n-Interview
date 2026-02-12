import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';

export default tseslint.config(
  js.configs.recommended,
  ...tseslint.configs.recommendedTypeChecked,
  ...svelte.configs['flat/recommended'],
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
        extraFileExtensions: ['.svelte'],
      },
    },
  },
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: ['.svelte'],
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
  {
    // Disable type-checked rules for Svelte files.
    // Reason: typescript-eslint's type-aware rules don't understand Svelte 5's runes ($state, $derived, etc.)
    // and report them as "error typed values". Type safety for Svelte files is still enforced by
    // svelte-check (npm run check) which properly understands the Svelte 5 compilation model.
    files: ['**/*.svelte'],
    rules: {
      '@typescript-eslint/no-unsafe-assignment': 'off',
      '@typescript-eslint/no-unsafe-call': 'off',
      '@typescript-eslint/no-unsafe-member-access': 'off',
      '@typescript-eslint/no-unsafe-return': 'off',
      '@typescript-eslint/no-unsafe-argument': 'off',
      '@typescript-eslint/no-redundant-type-constituents': 'off',
    },
  },
  {
    // Disable type-checked rules for config files (not part of tsconfig project)
    files: ['*.config.{js,ts}', '*.config.*.{js,ts}'],
    ...tseslint.configs.disableTypeChecked,
  },
  {
    ignores: ['dist/', 'build/', 'src-tauri/target/', 'node_modules/', '.svelte-kit/'],
  }
);
