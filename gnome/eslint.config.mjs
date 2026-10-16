import js from '@eslint/js';
import {defineConfig} from 'eslint/config';
import globals from 'globals';
import tseslint from 'typescript-eslint';

export default defineConfig(
    {
        files: ['extension/**/*.ts'],
        extends: [js.configs.recommended, tseslint.configs.strictTypeChecked],
        languageOptions: {
            parserOptions: {projectService: true, tsconfigRootDir: import.meta.dirname},
        },
        rules: {
            '@typescript-eslint/switch-exhaustiveness-check': 'error',
            '@typescript-eslint/restrict-template-expressions': ['error', {allowNumber: true}],
        },
    },
    {
        files: [
            'extension/extension.ts',
            'extension/src/**/*.ts',
            'extension/tests/integration/**/*.ts',
            'extension/tests/support/{actors,dbus,shell-events}.ts',
        ],
        languageOptions: {
            globals: {
                ARGV: 'readonly',
                Debugger: 'readonly',
                GIRepositoryGType: 'readonly',
                imports: 'readonly',
                log: 'readonly',
                logError: 'readonly',
                pkg: 'readonly',
                print: 'readonly',
                printerr: 'readonly',
                console: 'readonly',
                TextEncoder: 'readonly',
                TextDecoder: 'readonly',
                setTimeout: 'readonly',
                setInterval: 'readonly',
                clearTimeout: 'readonly',
                clearInterval: 'readonly',
                global: 'readonly',
                _: 'readonly',
                C_: 'readonly',
                N_: 'readonly',
                ngettext: 'readonly',
            },
        },
        rules: {
            'no-restricted-globals': [
                'error',
                {name: 'Debugger', message: 'Reserved for GJS internals.'},
                {name: 'GIRepositoryGType', message: 'Reserved for GJS internals.'},
                {name: 'log', message: 'Use console.log().'},
                {name: 'logError', message: 'Use console.warn() or console.error().'},
            ],
            'no-restricted-properties': [
                'error',
                {object: 'imports', property: 'format', message: 'Use template strings.'},
                {object: 'pkg', property: 'initFormat', message: 'Use template strings.'},
                {object: 'Lang', property: 'copyProperties', message: 'Use Object.assign().'},
                {object: 'Lang', property: 'bind', message: 'Use arrow functions or Function.prototype.bind().'},
                {object: 'Lang', property: 'Class', message: 'Use ES classes.'},
            ],
            'no-restricted-syntax': [
                'error',
                {
                    selector:
                        'MethodDefinition[key.name="_init"] CallExpression[arguments.length<=1][callee.object.type="Super"][callee.property.name="_init"]',
                    message: 'Use constructor() and super().',
                },
            ],
        },
    },
    {
        files: ['extension/tests/*.test.ts', 'extension/tests/support/{coverage,coverage.test,node,state}.ts'],
        languageOptions: {
            globals: globals.node,
            parserOptions: {projectService: false, project: './tsconfig.node.json'},
        },
    },
    {
        files: ['**/*.mjs'],
        extends: [js.configs.recommended],
        languageOptions: {globals: globals.node},
    },
);
