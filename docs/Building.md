## Prerequisites

Before building, ensure you have the following requirements installed:

- **[Node.js](https://nodejs.org/)** - JavaScript/TypeScript runtime (Required for frontend dependencies)
- **[Deno](https://deno.land/)** - JavaScript/TypeScript runtime (Preferred build tool)
- **[Rust](https://www.rust-lang.org/)** - System programming language (1.90+)
- **PC/SC Middleware**:
  - Linux: `pcscd` (usually pre-installed)
  - macOS: Built-in
  - Windows: Built-in

> [!IMPORTANT]
> **Deno is required** to build the application as intended. Node.js is also required because some frontend dependencies rely on it. The npm commands provided in this guide assume Deno is installed and are provided as a fallback.

## Building from Source

### 1. Build the Application

#### Development Build

**With Deno (Recommended):**
```bash
deno task tauri dev
```

**With npm:**
```bash
npm run tauri dev
```

#### Production Build

**With Deno (Recommended):**
```bash
deno task tauri build
```

**With npm:**
```bash
npm run tauri build
```

The compiled binaries will be available in:
- **Linux**: `src-tauri/target/release/bundle/`
- **macOS**: `src-tauri/target/release/bundle/dmg/`
- **Windows**: `src-tauri/target/release/bundle/`

### Alternate Method: Pure npm Build (Not Recommended)

If you must use npm exclusively (without Deno installed), you can modify `src-tauri/tauri.conf.json` to change the build commands:

```json
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../build"
  },
```

After this change, you can use:
```bash
npm run build
npm run tauri build
```

> [!CAUTION]
> This deviation from the standard build process is **not recommended** for contributors. The project is designed to be built using Deno. The `package-lock.json` and npm commands exist primarily to facilitate packaging for Nix.

## Building and Development with Nix

[Nix](https://nixos.org/) provides developers with a complete and consistent development environment.

You can use Nix to build and develop picoforge painlessly.

### 1. Install Nix

Follow the [Installation Guide](https://nixos.org/download/#download-nix) and [NixOS Wiki](https://wiki.nixos.org/wiki/Flakes#Setup) to install Nix and enable Flakes.

### 2. Build & Run

#### a. with Flakes

You can build and run PicoForge with a single command:

```bash
nix run github:librekeys/picoforge
```

Or simply build it and link to the current directory:

```bash
nix build github:librekeys/picoforge
```

**Tip:** You can use our binary cache to save build time by allowing Nix to set extra-substitutes.

#### b. without Flakes

Download the package definition:

```bash
curl -LO https://raw.githubusercontent.com/librekeys/picoforge/main/package.nix
```

Run the following command in the directory containing `package.nix`:

```bash
nix-build -E 'with import <nixpkgs> {}; callPackage ./package.nix { }'
```

The compiled binary will be available at: `result/bin/picoforge`

### 3. Develop

You can enter a developement environement with all the required dependencies.

#### a. with Flakes

```bash
nix develop github:librekeys/picoforge
```

#### b. without Flakes

You can use the `shell.nix` file that is at the root of the repository by running:

```bash
nix-shell
```

Then you can build from source and run the application with:

```bash
deno task tauri dev
```
