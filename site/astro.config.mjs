// @ts-check
import casoonPages from '@casoon/pages-theme';
import { defineConfig } from 'astro/config';

// Project page: https://casoon.github.io/origin/ — `base` is the GitHub Pages path.
export default defineConfig({
  site: 'https://casoon.github.io/origin',
  base: '/origin/',
  integrations: [
    casoonPages({
      name: 'origin',
      description:
        'A reference architecture and starter system for modular desktop applications built with Rust and Tauri.',
      tagline: 'Modular Rust and Tauri desktop apps',
      website: 'https://origin.casoon.de',
      repo: 'casoon/origin',
      license: 'MIT',
      packages: [
        { label: 'crates.io', href: 'https://crates.io/crates/origin-app' },
        { label: 'docs.rs', href: 'https://docs.rs/origin-app' },
        { label: 'npm', href: 'https://www.npmjs.com/package/@casoon/origin-client' },
      ],
      docsGroups: {
        'getting-started': 'Getting started',
        concepts: 'Concepts',
        guides: 'Guides',
        lifecycle: 'Project lifecycle',
        reference: 'Reference',
      },
      mdxComponents: './src/mdx-components.ts',
    }),
  ],
});
