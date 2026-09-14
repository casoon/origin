import { readFileSync } from 'node:fs';
import { ansiToHtml } from '@casoon/pages-theme/ansi';
import type { ShowcaseExample } from '@casoon/pages-theme/showcase';
import appToml from '../../examples/demo/app.toml?raw';
import capability from '../../examples/demo/src-tauri/capabilities/standard-dashboard.json?raw';
import pulseRs from '../../examples/demo/src-tauri/src/pulse.rs?raw';

// Read, not imported with ?raw: Vite would transform the .ts file and resolve the demo's
// tsconfig (extends @tsconfig/svelte), which is not installed in the Pages build.
// Relative to site/, where the build runs. Also used by the start page.
export const pulseTs = readFileSync('../examples/demo/src/pulse.generated.ts', 'utf8');

// Both pairs are committed files of the reference application. CI keeps the generated
// halves current: `cargo xtask generate --check` for the capability file, `cargo test`
// for the TypeScript binding. The site only reads them.

// The declaration is cut from the module source, so it cannot drift from the file.
const snapshot = pulseRs.match(/#\[derive\([^\n]*ts_rs::TS\)\]\npub struct PulseSnapshot \{[\s\S]*?\n\}/)?.[0];
if (!snapshot) throw new Error('PulseSnapshot not found in examples/demo/src-tauri/src/pulse.rs');

export const examples: ShowcaseExample[] = [
  {
    slug: 'manifest-to-capability',
    title: 'Manifest to capability file',
    file: 'examples/demo/app.toml',
    tags: ['app.toml', 'security profile', 'cargo xtask generate'],
    description:
      'The demo’s app.toml assigns the main window the standard-dashboard profile. cargo xtask generate derives src-tauri/capabilities/standard-dashboard.json from it: explicit permissions, no filesystem, shell or process access.',
    input: { code: appToml, lang: 'toml' },
    output: { html: ansiToHtml(capability), kind: 'terminal' },
  },
  {
    slug: 'rust-to-typescript',
    title: 'Rust type to TypeScript binding',
    file: 'examples/demo/src-tauri/src/pulse.rs',
    tags: ['ts-rs', 'IPC contract', 'cargo test'],
    description:
      'PulseSnapshot is what the demo’s UI renders. Its TypeScript type in src/pulse.generated.ts is derived from the Rust struct with ts-rs, and a test fails when the checked-in file drifts.',
    input: { code: snapshot, lang: 'rust' },
    output: { html: ansiToHtml(pulseTs), kind: 'terminal' },
  },
];
