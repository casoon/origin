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
      repo: 'casoon/origin',
      version: '0.2.0',
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
      // No CHANGELOG.md yet; v0.2.0 is the only tag.
      changelog: false,
    }),
  ],
});
