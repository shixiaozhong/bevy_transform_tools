# bevy_transform_tools

Bevy-based 3D model transform viewer intended for WebAssembly embedding.

## Current Capabilities

- Imports OBJ text through `tobj`.
- Imports STL bytes through `stl_io`.
- Supports ASCII STL and binary STL.
- Spawns imported meshes as Bevy `Mesh3d` entities.
- Shows a basic X/Y/Z transform gizmo for the selected model.
- Supports mouse interaction:
  - Left-drag a gizmo axis to translate the selected model.
  - Right-drag to orbit the camera.
  - Mouse wheel to zoom.
- Exposes wasm-bindgen functions for web callers.

## Exported Web API

The wasm build exports these functions:

- `load_obj_model(name: string, source: string): number`
- `load_stl_model(name: string, bytes: Uint8Array): number`
- `select_model_by_id(id: number): void`
- `clear_scene_models(): void`
- `get_selected_model_id(): number`
- `get_model_transform_json(id: number): string`
- `get_last_error(): string`
- `set_transform(id, tx, ty, tz, rx, ry, rz, sx, sy, sz): void`

`rx`, `ry`, and `rz` are radians using XYZ Euler order. `get_model_transform_json`
returns `null` when the id is unknown, otherwise:

```json
{"translation":[0,0,0],"rotation_xyzw":[0,0,0,1],"scale":[1,1,1]}
```

## Build For Web

Install the wasm target and wasm-bindgen CLI if needed:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

Build and generate web bindings:

```sh
cargo build --release --target wasm32-unknown-unknown --lib
wasm-bindgen \
  --out-dir web/pkg \
  --target web \
  target/wasm32-unknown-unknown/release/bevy_transform_tools.wasm
```

Then serve the `web` directory with any static file server.

## Local Checks

```sh
cargo test
cargo check
cargo check --target wasm32-unknown-unknown
```
