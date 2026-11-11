// ESLint flat config (stack/backend-lambda.md). Never disable a rule to go green.
import js from '@eslint/js';
import importX from 'eslint-plugin-import-x';
import jsdoc from 'eslint-plugin-jsdoc';
import globals from 'globals';
import tseslint from 'typescript-eslint';

const SIZE_RULES = {
  'max-lines': ['error', { max: 300, skipBlankLines: true, skipComments: true }],
  'max-lines-per-function': ['error', { max: 40, skipBlankLines: true, skipComments: true }],
  'max-params': ['error', 4],
  complexity: ['error', 10],
};

export default tseslint.config(
  { ignores: ['node_modules', 'dist', 'coverage', 'terraform', '.dependency-cruiser.cjs'] },
  js.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  {
    languageOptions: {
      globals: globals.node,
      parserOptions: { projectService: true, tsconfigRootDir: import.meta.dirname },
    },
    plugins: { 'import-x': importX, jsdoc },
    rules: {
      ...SIZE_RULES,
      'no-console': 'error',
      'import-x/no-cycle': 'error',
      'import-x/no-default-export': 'error',
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-floating-promises': 'error',
      '@typescript-eslint/consistent-type-definitions': ['error', 'interface'],
      'no-restricted-syntax': [
        'error',
        { selector: 'TSEnumDeclaration', message: 'Use a string-literal union, not enum.' },
      ],
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: ['@aws-sdk/*', '@anthropic-ai/*'],
              message: 'Only src/adapters and src/composition may import SDKs.',
            },
            {
              group: ['**/v[0-9]*', '**/v[0-9]*.js'],
              message: 'Import the unit pointer (index.ts), never a v<N> file.',
            },
          ],
        },
      ],
      'jsdoc/require-jsdoc': [
        'error',
        {
          publicOnly: true,
          require: {
            FunctionDeclaration: true,
            ArrowFunctionExpression: true,
            ClassDeclaration: true,
          },
        },
      ],
      'jsdoc/no-types': 'error',
    },
  },
  {
    // The composition root may import a v<N> file directly (runtime version switch).
    files: ['src/composition/**'],
    rules: { 'no-restricted-imports': 'off' },
  },
  {
    files: ['src/adapters/**'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            { group: ['**/v[0-9]*', '**/v[0-9]*.js'], message: 'Import the unit pointer.' },
          ],
        },
      ],
    },
  },
  {
    files: ['src/**/index.ts', 'src/**/*.test.ts', 'src/**/__tests__/**', 'test/**'],
    rules: { 'no-restricted-imports': 'off', 'max-lines-per-function': 'off' },
  },
  {
    files: ['scripts/**/*.mjs', 'eslint.config.js', 'vitest.config.ts', 'vitest.it.config.ts'],
    ...tseslint.configs.disableTypeChecked,
  },
  {
    // Build scripts are CLIs: stdout is their interface, not a log.
    files: ['scripts/**/*.mjs'],
    rules: { 'no-console': 'off' },
  },
  {
    files: ['eslint.config.js', 'vitest.config.ts', 'vitest.it.config.ts'],
    rules: { 'import-x/no-default-export': 'off' },
  },
);
