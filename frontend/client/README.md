# @casoon/origin-client

Typed transport layer between an [Origin](https://github.com/casoon/origin) frontend and
the Rust core. It is the only package that imports `@tauri-apps/api`; views import typed
functions from here instead (ADR-0010).

```ts
import { settings } from "@casoon/origin-client";

const theme = await settings.get<string>("theme");
```

The contract types are generated from the Rust definitions by `cargo xtask generate`
and ship with the package — products do not regenerate them.

The package ships TypeScript sources and expects a bundler that compiles them (Vite in
every Origin project).

License: MIT
