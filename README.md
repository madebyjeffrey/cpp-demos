# Skia Window

A C++23 static library for native macOS windows, per-window menus, and modal/modeless dialogs, with **Skia Ganesh rendering directly into Metal drawables**. Includes an interactive GPU gallery.

The initial backend requires **macOS 26 or later**. Apple Silicon and Intel are selectable build architectures; the implementation has been exercised on Apple Silicon. Windows/Linux backends are not implemented.

## Build and run

Prerequisites: Xcode with its command-line tools selected, CMake 3.28+ (3.25+ with SCARS disabled), Apple Clang 21+ for SCARS, Ninja, Python 3, Git, and an internet connection for the first dependency bootstrap. Use the current Xcode C++23 compiler and macOS SDK. No package manager or prebuilt Skia binary is required.

```sh
python3 scripts/bootstrap_skia.py --config RelWithDebInfo
python3 scripts/bootstrap_scars_text.py --config RelWithDebInfo
cmake --preset relwithdebinfo
cmake --build --preset relwithdebinfo
open build/relwithdebinfo/skia_window_demo.app
```

For Debug, first build the matching Skia configuration:

```sh
python3 scripts/bootstrap_skia.py --config Debug
python3 scripts/bootstrap_scars_text.py --config Debug
cmake --preset debug
cmake --build --preset debug
```

Open the root `CMakeLists.txt` in CLion and select either supplied CMake preset. Launch the `skia_window_demo` target. Building and running the bundle require no network access after bootstrap.

`dependencies/skia.json` locks the source revision, minimum OS, and GN settings. The bootstrap uses Skia's `tools/git-sync-deps`, GN, and Ninja, skips Emscripten activation, and caches downloads and per-architecture/configuration builds under `.deps/`. Repeated invocations reuse them; `--resync` rechecks all source dependencies. It refuses to replace tracked local edits in the Skia checkout. CMake never downloads dependencies.

For an Intel build from either host architecture:

```sh
python3 scripts/bootstrap_skia.py --arch x86_64 --config RelWithDebInfo
python3 scripts/bootstrap_scars_text.py --arch x86_64 --config RelWithDebInfo
cmake -S . -B build/intel -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo -DCMAKE_OSX_ARCHITECTURES=x86_64
cmake --build build/intel
```

Build one architecture at a time. `SKIA_SOURCE_DIR` and `SKIA_LIBRARY_DIR` can point to another checkout/build produced by the same bootstrap. Configuration validates the lock revision, architecture, configuration, and deployment target against its manifest.

## SCARS: independent module-based controls

A Rust windowing and reactive component workspace is available in [`rust/`](rust/README.md).
It provides AppKit windows/dialogs, per-window menus, native light/dark and full-screen
titlebar buttons, input/IME, and Skia canvases backed by Metal without
depending on the C++ build. Cargo builds Skia from source through `skia-safe` /
`skia-bindings`. Run it with `cd rust` then `cargo run --locked --example windows`;
see the Rust README for source-build prerequisites. The [Rust quick guide](docs/rust-guide.md)
covers the staged examples, component contracts, and themed todo app.

[SCARS API quick guide](docs/scars-api.md) · [Build and detailed contracts](docs/scars.md) · [Control implementation plan](docs/scars-controls-plan.md)

**Structured Control and Rendering System (SCARS)** adds a separate C++23 engine and **SCARS Lab** app, with LCARS-inspired flat dark controls, chained view modifiers, observable state/bindings, flex/block/inline layouts, and SkParagraph text layout. It does not wrap the existing declarative UI engine.

With Apple Clang 21+ and CMake 3.28+, build the additional pinned text stack before configuring SCARS:

```sh
python3 scripts/bootstrap_scars_text.py --config RelWithDebInfo
cmake --preset relwithdebinfo
cmake --build --preset relwithdebinfo --target scars_lab
open build/relwithdebinfo/scars/scars_lab.app
```

The original renderer must already be bootstrapped for the same configuration. Use `Debug` and the `debug` preset for Debug builds. Press **Command-Option-I** in SCARS Lab for the floating element explorer, with live tree/layout/state details and click-to-pick inspection. To configure only the original library/demos, pass `-DSCARS_BUILD=OFF`. See [the SCARS guide](docs/scars.md) for module imports, view examples, layout/text contracts, compiler targets, and validation commands.

## Declarative controls and layout

The app now also opens a **Controls Lab** demonstrating SwiftUI-inspired C++ view definitions, reactive state/bindings, responsive flex/grid/overlay layout, buttons, checkboxes, sliders, scrolling, custom Skia canvases, nested view identity, and interruptible transitions. Its modal dialog shares bindings with the parent. Reopen it from the original gallery with **Command-L**.

See [the declarative UI guide](docs/ui.md) for the interface, examples, layout contracts, control/accessibility behavior, and tests. The original low-level window/canvas interface remains available.

## Try the gallery

Six cards demonstrate paths, animated geometry, gradient interpolation, transforms, text, image textures, clipping, blurred shadows, blend modes, and a custom SkSL shader. Hover a card to highlight it; scroll to adjust animation speed.

| Action | Shortcut |
| --- | --- |
| New independent window | Command-N |
| Close window | Command-W |
| Animate / pause | Command-P or Space |
| Reset time and speed | Command-R or R |
| Modeless inspector | Command-I |
| Modal example sheet | Command-D |
| Accept / cancel a sheet | Enter / Escape |
| Quit | Command-Q |

Menus follow the focused window. Dialogs retain their parent's menu. A sheet blocks its parent while other windows continue working. Closing all windows leaves the application running; use the application menu's New Window command or reopen the app from the Dock.

The displayed FPS and frame interval describe draw-callback cadence, **not GPU execution time**. Paused content redraws only when invalidated. Startup shader compilation can temporarily increase frame intervals.

## Use from another C++ project

```cmake
cmake_minimum_required(VERSION 3.25)
project(example LANGUAGES CXX)
set(SKIA_WINDOW_BUILD_DEMO OFF CACHE BOOL "" FORCE)
set(SKIA_WINDOW_BUILD_TESTS OFF CACHE BOOL "" FORCE)
add_subdirectory(path/to/cpp-demos skia-window)
add_executable(example main.cpp)
target_link_libraries(example PRIVATE skia_window::skia_window)
```

Configure with Ninja and `-DCMAKE_BUILD_TYPE=Debug` or `RelWithDebInfo`. Only the backend builds Objective-C++; your sources and public library headers remain C++. The target propagates C++23, Skia headers/libraries, and required system frameworks.

```cpp
#include <skia_window/skia_window.hpp>
#include "include/core/SkCanvas.h"
#include "include/core/SkPaint.h"
#include <iostream>

int main() {
    using namespace skia_window;
    auto app_result = Application::create({.name = "Example"});
    if (!app_result) {
        std::cerr << app_result.error().message << '\n';
        return 1;
    }
    auto app = std::move(*app_result);
    auto window_result = Window::create(*app, {.title = "Hello Metal"});
    if (!window_result) {
        std::cerr << window_result.error().message << '\n';
        return 1;
    }
    auto window = std::move(*window_result);
    window->set_callbacks({.on_draw = [](SkCanvas& canvas, const FrameInfo&) {
        canvas.clear(0xff10191f);
        SkPaint paint;
        paint.setAntiAlias(true);
        paint.setColor(0xff66e0c2);
        canvas.drawCircle(160, 130, 70, paint);
    }});
    window->show();
    return app->run();
}
```

See [the interface guide](docs/interface.md) for lifecycle and rendering contracts, [validation notes](docs/validation.md) for checks and diagnostic capture commands, and `demo/main.cpp` for menus and both dialog types.

## Assets and dependencies

Roboto Regular is bundled for offline text rendering: Copyright 2011 Google Inc., Apache License 2.0. Its license is included in `demo/assets/Roboto-LICENSE.txt` and copied into the app. The original PNG texture is reproducible with `python3 scripts/generate_texture.py`.

Skia uses the BSD license reproduced in `dependencies/Skia-LICENSE.txt`; its upstream source and third-party dependencies retain their own licenses in `.deps/skia`. Review and distribute applicable notices when packaging a downstream application.
