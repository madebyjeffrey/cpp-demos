#import <AppKit/AppKit.h>
#import <Carbon/Carbon.h>
#import <Metal/Metal.h>
#import <QuartzCore/CAMetalLayer.h>
#import <QuartzCore/CADisplayLink.h>
#import <QuartzCore/CATransaction.h>

#include <skia_window/skia_window.hpp>
#include <skia_window/ui.hpp>
#include "include/core/SkFontMgr.h"
#include "include/ports/SkFontMgr_mac_ct.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkColorSpace.h"
#include "include/core/SkSurface.h"
#include "include/gpu/ganesh/GrBackendSurface.h"
#include "include/gpu/ganesh/GrDirectContext.h"
#include "include/gpu/ganesh/SkSurfaceGanesh.h"
#include "include/gpu/ganesh/mtl/GrMtlBackendContext.h"
#include "include/gpu/ganesh/mtl/GrMtlBackendSurface.h"
#include "include/gpu/ganesh/mtl/GrMtlDirectContext.h"

#include <algorithm>
#include <cassert>
#include <cmath>
#include <cstdlib>
#include <exception>
#include <format>
#include <iostream>
#include <stdexcept>
#include <unordered_set>

namespace sw = skia_window;
namespace skia_window::detail { struct SurfaceState; struct ApplicationState; }

@interface SWView : NSView <NSTextInputClient, NSUserInterfaceValidations> {
@public
    std::weak_ptr<sw::detail::SurfaceState> state;
    NSTrackingArea* tracking;
    sw::Cursor pointer_cursor;
    NSMutableDictionary<NSString*, NSAccessibilityElement*>* accessibility_nodes;
}
- (void)tick:(CADisplayLink*)sender;
@end
@interface SWAccessibilityElement : NSAccessibilityElement {
@public
    std::weak_ptr<sw::detail::SurfaceState> state;
    std::string element_id;
}
@end
@interface SWExternalAccessibilityElement : NSAccessibilityElement {
@public
    std::weak_ptr<sw::detail::SurfaceState> state;
    std::string element_id;
}
@end
// NSArray primitives keep logical row access lazy. Only mounted elements live
// in the view's cache; an accessibility client may retain individual row objects.
@interface SWLogicalAccessibilityRows : NSArray {
@public
    std::weak_ptr<sw::detail::SurfaceState> state;
    std::string list_id;
    bool selected_only, children_only;
    unsigned cell_mode; // 1 row cells, 2 column cells, 3 selected-row cells, 4 columns
}
@end
@interface SWLogicalAccessibilityObjects : NSArray {
@public
    std::weak_ptr<sw::detail::SurfaceState> state;
    std::vector<std::string> ids;
}
@end
@interface SWWindowDelegate : NSObject <NSWindowDelegate> {
@public
    std::weak_ptr<sw::detail::SurfaceState> state;
}
@end
@interface SWMenuTarget : NSObject {
@public
    std::weak_ptr<sw::detail::SurfaceState> state;
    std::weak_ptr<sw::detail::ApplicationState> application;
}
- (void)command:(NSMenuItem*)sender;
- (void)newWindow:(id)sender;
- (void)themeToggle:(NSButton*)sender;
@end
@interface SWApplicationDelegate : NSObject <NSApplicationDelegate> {
@public
    std::weak_ptr<sw::detail::ApplicationState> state;
}
@end

namespace skia_window::detail {
using Clock = std::chrono::steady_clock;
sk_sp<SkTypeface> default_typeface() {
    return SkFontMgr_New_CoreText(nullptr)->matchFamilyStyle(nullptr, SkFontStyle{});
}

struct ApplicationState : std::enable_shared_from_this<ApplicationState> {
    ApplicationOptions options;
    id<MTLDevice> device;
    id<MTLCommandQueue> queue;
    sk_sp<GrDirectContext> context;
    SWApplicationDelegate* delegate;
    SWMenuTarget* target;
    NSMenu* default_menu;
    id motion_observer;
    std::vector<std::weak_ptr<SurfaceState>> surfaces;
    Clock::time_point epoch{Clock::now()};
    bool alive{true}, running{}, capture_active{};
    int exit_code{};
    ~ApplicationState() {
        if (motion_observer) [NSWorkspace.sharedWorkspace.notificationCenter removeObserver:motion_observer];
        if (capture_active) [[MTLCaptureManager sharedCaptureManager] stopCapture];
    }
};
struct SurfaceState : std::enable_shared_from_this<SurfaceState> {
    std::weak_ptr<ApplicationState> application;
    std::weak_ptr<SurfaceState> parent;
    std::vector<std::weak_ptr<SurfaceState>> children;
    NSWindow* window;
    SWView* view;
    SWWindowDelegate* delegate;
    SWMenuTarget* target;
    CAMetalLayer* layer;
    CADisplayLink* display_link;
    NSMenu* menu;
    std::vector<MenuItem> menu_items;
    std::function<void(Command)> on_command;
    Callbacks callbacks;
    std::shared_ptr<ui::Host> ui;
    std::shared_ptr<AccessibilityProvider> accessibility;
    std::shared_ptr<TextInputClient> text_input;
    bool secure_input_enabled{};
    NSTitlebarAccessoryViewController* theme_accessory;
    NSButton* theme_button;
    std::function<void(bool)> theme_changed;
    std::uint64_t ui_generation{}, ui_mount_epoch{}, accessibility_generation{};
    std::uint64_t redraw_timer_generation{};
    std::function<void(DialogResult)> completion;
    bool dialog{}, open{true}, dirty{true}, animating{}, modal{}, closing{}, rendering{}, failed{};
    unsigned unavailable_frames{};
    Size synced_size{-1, -1};
    double synced_scale{};
    Clock::time_point previous{};
    std::uint64_t frame_number{};
    std::string title;
};

static void main_thread() {
    if (![NSThread isMainThread]) throw std::logic_error("skia_window requires the main thread");
}
static NSString* ns(const std::string& value) {
    NSString* result = [[NSString alloc] initWithBytes:value.data() length:value.size() encoding:NSUTF8StringEncoding];
    return result ? result : @"";
}
static void report(const std::shared_ptr<ApplicationState>& app, std::string message) noexcept {
    if (!app || !app->alive) return;
    try {
        auto handler = app->options.on_error;
        if (handler) handler(Error{std::move(message)});
        else std::cerr << "skia_window: " << message << '\n';
    } catch (...) {
        std::cerr << "skia_window: error handler threw an exception\n";
    }
}
// Every entry from Objective-C/dispatch goes through this exception barrier.
static void guarded(const std::shared_ptr<ApplicationState>& app, const auto& function) noexcept {
    if (!app || !app->alive) return;
    try { function(); }
    catch (const std::exception& error) { report(app, error.what()); }
    catch (...) { report(app, "Unhandled exception in application callback"); }
}
static void guarded(const std::shared_ptr<SurfaceState>& surface, const auto& function) noexcept {
    if (surface) guarded(surface->application.lock(), function);
}
static void sync_secure_input(const std::shared_ptr<SurfaceState>&, bool force_off = false);
static void emit(const std::shared_ptr<SurfaceState>& surface, const Event& event) {
    if (!surface->open) return;
    auto host = surface->ui;
    const bool consumed = host && host->handle(event);
    if (!surface->open || consumed) return;
    auto callback = surface->callbacks.on_event;
    if (callback) callback(event);
    sync_secure_input(surface);
    const auto* pointer = std::get_if<PointerEvent>(&event);
    if (surface->open && surface->accessibility &&
        (std::holds_alternative<KeyEvent>(event) || std::holds_alternative<FocusEvent>(event) ||
         (pointer && (pointer->kind == PointerKind::down || pointer->kind == PointerKind::up))))
        NSAccessibilityPostNotification(surface->view, NSAccessibilityFocusedUIElementChangedNotification);
}
static Modifiers modifiers(NSEventModifierFlags flags) {
    return {.shift = bool(flags & NSEventModifierFlagShift),
            .control = bool(flags & NSEventModifierFlagControl),
            .alt = bool(flags & NSEventModifierFlagOption),
            .command = bool(flags & NSEventModifierFlagCommand)};
}
static NSEventModifierFlags flags(Modifiers mods) {
    return (mods.shift ? NSEventModifierFlagShift : 0) |
           (mods.control ? NSEventModifierFlagControl : 0) |
           (mods.alt ? NSEventModifierFlagOption : 0) |
           (mods.command ? NSEventModifierFlagCommand : 0);
}
static Key key(NSEvent* event) {
    switch (event.keyCode) {
        case 53: return Key::escape; case 36: case 76: return Key::enter;
        case 48: return Key::tab; case 49: return Key::space; case 51: return Key::backspace;
        case 117: return Key::delete_forward; case 123: return Key::left; case 124: return Key::right;
        case 125: return Key::down; case 126: return Key::up; case 115: return Key::home;
        case 119: return Key::end; case 116: return Key::page_up; case 121: return Key::page_down;
        case 122: return Key::f1; case 120: return Key::f2; case 99: return Key::f3;
        case 118: return Key::f4; case 96: return Key::f5; case 97: return Key::f6;
        case 98: return Key::f7; case 100: return Key::f8; case 101: return Key::f9;
        case 109: return Key::f10; case 103: return Key::f11; case 111: return Key::f12;
        default: break;
    }
    NSString* chars = event.charactersIgnoringModifiers.lowercaseString;
    if (chars.length == 1) {
        auto c = [chars characterAtIndex:0];
        if (c >= 'a' && c <= 'z') return static_cast<Key>(std::to_underlying(Key::a) + c - 'a');
        if (c >= '0' && c <= '9') return static_cast<Key>(std::to_underlying(Key::num0) + c - '0');
    }
    return Key::unknown;
}
static NSString* equivalent(Key key) {
    const auto value = std::to_underlying(key);
    if (key >= Key::a && key <= Key::z) return [NSString stringWithFormat:@"%c", 'a' + value - std::to_underlying(Key::a)];
    if (key >= Key::num0 && key <= Key::num9) return [NSString stringWithFormat:@"%c", '0' + value - std::to_underlying(Key::num0)];
    if (key >= Key::f1 && key <= Key::f12) {
        unichar c = NSF1FunctionKey + value - std::to_underlying(Key::f1);
        return [NSString stringWithCharacters:&c length:1];
    }
    switch (key) {
        case Key::enter: return @"\r"; case Key::escape: return @"\033";
        case Key::space: return @" "; case Key::tab: return @"\t";
        case Key::backspace: return @"\010"; case Key::delete_forward: return @"\177";
        case Key::left: return @"\uF702"; case Key::right: return @"\uF703";
        case Key::up: return @"\uF700"; case Key::down: return @"\uF701";
        case Key::home: return @"\uF729"; case Key::end: return @"\uF72B";
        case Key::page_up: return @"\uF72C"; case Key::page_down: return @"\uF72D";
        default: return @"";
    }
}
static Size content_size(const std::shared_ptr<SurfaceState>& surface) {
    return {surface->view.bounds.size.width, surface->view.bounds.size.height};
}
static double scale(const std::shared_ptr<SurfaceState>& surface) {
    return surface->window.backingScaleFactor;
}
static bool sync_size(const std::shared_ptr<SurfaceState>& surface) {
    if (!surface->open) return false;
    const auto size = content_size(surface);
    const auto s = scale(surface);
    if (surface->synced_size.width == size.width && surface->synced_size.height == size.height &&
        surface->synced_scale == s) return false;
    surface->synced_size = size;
    surface->synced_scale = s;
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    surface->layer.contentsScale = s;
    surface->layer.drawableSize = CGSizeMake(std::round(size.width * s), std::round(size.height * s));
    [CATransaction commit];
    surface->dirty = true;
    emit(surface, ResizeEvent{size, {surface->layer.drawableSize.width, surface->layer.drawableSize.height}, s});
    return true;
}
static void activate_menu(const std::shared_ptr<SurfaceState>& surface) {
    auto owner = surface->dialog ? surface->parent.lock() : surface;
    if (auto app = surface->application.lock()) NSApp.mainMenu = owner && owner->menu ? owner->menu : app->default_menu;
}
static bool interactive(const std::shared_ptr<SurfaceState>& surface) {
    return surface->open && !surface->window.attachedSheet;
}
static MenuItem* find_command(std::vector<MenuItem>& items, Command command, bool ancestors_enabled = true,
                              bool* enabled = nullptr) {
    for (auto& item : items) {
        const bool allowed = ancestors_enabled && item.enabled;
        if (item.command == command && command != 0) {
            if (enabled) *enabled = allowed;
            return &item;
        }
        if (auto* found = find_command(item.children, command, allowed, enabled)) return found;
    }
    return nullptr;
}
static bool invoke(const std::shared_ptr<SurfaceState>& surface, Command command) {
    bool enabled = false;
    if (!interactive(surface) || !find_command(surface->menu_items, command, true, &enabled) || !enabled) return false;
    auto callback = surface->on_command;
    if (!callback) return false;
    guarded(surface, [&] { callback(command); });
    return true;
}
static NSMenuItem* native_command(NSMenu* menu, Command command) {
    for (NSMenuItem* item in menu.itemArray) {
        if (item.tag == command && item.action == @selector(command:)) return item;
        if (item.submenu) if (auto child = native_command(item.submenu, command)) return child;
    }
    return nil;
}
static void close_surface(std::shared_ptr<SurfaceState> surface, DialogResult result = DialogResult::cancelled);
static void update_pause(const std::shared_ptr<SurfaceState>& surface) {
    const bool visible = surface->open && surface->window.visible && !surface->window.miniaturized &&
        (surface->window.occlusionState & NSWindowOcclusionStateVisible);
    surface->display_link.paused = !visible || surface->failed || (!surface->animating && !surface->dirty && !(surface->ui && surface->ui->needs_frame()));
    if (!visible) surface->previous = {};
}
static void render(const std::shared_ptr<SurfaceState>& surface, bool sizing_transaction = false) {
    auto app = surface->application.lock();
    if (!app || !surface->open || surface->rendering || surface->failed) return;
    update_pause(surface);
    if (surface->display_link.paused) return;
    if (app->context->abandoned()) {
        surface->failed = true;
        update_pause(surface);
        report(app, "The Metal Skia context was lost");
        return;
    }
    const auto dimensions = surface->layer.drawableSize;
    if (dimensions.width <= 0 || dimensions.height <= 0) return;
    @autoreleasepool {
        // Resize content must commit with the new layer bounds. Normal vsync frames keep async
        // presentation; waiting for scheduling is confined to AppKit sizing transactions.
        CAMetalLayer* layer = surface->layer;
        const bool previous_presentation = layer.presentsWithTransaction;
        layer.presentsWithTransaction = sizing_transaction;
        struct RestorePresentation {
            CAMetalLayer* __strong layer;
            bool previous;
            ~RestorePresentation() { layer.presentsWithTransaction = previous; }
        } restore_presentation{layer, previous_presentation};
        id<CAMetalDrawable> drawable = [surface->layer nextDrawable];
        if (!drawable) {
            if (++surface->unavailable_frames >= 120) {
                surface->failed = true;
                update_pause(surface);
                report(app, "Metal drawables remain unavailable; rendering stopped");
            }
            return;
        }
        surface->unavailable_frames = 0;
        GrMtlTextureInfo texture;
        texture.fTexture.retain((__bridge GrMTLHandle)drawable.texture);
        const auto target = GrBackendRenderTargets::MakeMtl((int)dimensions.width, (int)dimensions.height, texture);
        auto sk_surface = SkSurfaces::WrapBackendRenderTarget(app->context.get(), target,
            kTopLeft_GrSurfaceOrigin, kBGRA_8888_SkColorType, SkColorSpace::MakeSRGB(), nullptr);
        if (!sk_surface) {
            surface->failed = true;
            update_pause(surface);
            report(app, "Cannot wrap the Metal drawable in a Skia GPU surface");
            return;
        }
        surface->dirty = false;
        surface->rendering = true;
        struct Reset { bool& flag; ~Reset() { flag = false; } } reset{surface->rendering};
        auto now = Clock::now();
        const double s = scale(surface);
        FrameInfo frame{content_size(surface), {dimensions.width, dimensions.height}, s,
                        now - app->epoch, surface->previous == Clock::time_point{} ? Clock::duration{} : now - surface->previous,
                        surface->frame_number++};
        surface->previous = now;
        auto* canvas = sk_surface->getCanvas();
        canvas->clear(SK_ColorTRANSPARENT);
        const auto save_count = canvas->save();
        canvas->scale((float)s, (float)s);
        auto draw = surface->callbacks.on_draw;
        try {
            if (draw) draw(*canvas, frame);
            sync_secure_input(surface);
            if (surface->open && surface->accessibility && surface->accessibility->generation) {
                auto generation = surface->accessibility->generation();
                if (generation != surface->accessibility_generation) {
                    surface->accessibility_generation = generation;
                    NSAccessibilityPostNotification(surface->view, NSAccessibilityLayoutChangedNotification);
                }
            }
            auto host = surface->ui;
            if (host && surface->open) {
                host->paint(*canvas, frame);
                if (surface->open && surface->view && host->generation() != surface->ui_generation) {
                    surface->ui_generation = host->generation();
                    NSAccessibilityPostNotification(surface->view, NSAccessibilityLayoutChangedNotification);
                }
            }
        } catch (...) {
            // Restore state even when user rendering throws. Do not retry a failing callback at vsync.
            canvas->restoreToCount(save_count);
            surface->failed = true;
            update_pause(surface);
            throw;
        }
        canvas->restoreToCount(save_count);
        const auto flushed = app->context->flushAndSubmit(sk_surface.get());
        if (!flushed.fSuccess) {
            surface->failed = true;
            update_pause(surface);
            report(app, "Skia failed to submit its Metal rendering commands");
            return;
        }
        id<MTLCommandBuffer> command_buffer = [app->queue commandBuffer];
        if (!command_buffer) {
            surface->failed = true;
            update_pause(surface);
            report(app, "Metal failed to create the presentation command buffer");
            return;
        }
        command_buffer.label = @"skia_window present";
        if (!sizing_transaction) [command_buffer presentDrawable:drawable];
        std::weak_ptr<ApplicationState> weak_app = app;
        [command_buffer addCompletedHandler:^(id<MTLCommandBuffer> buffer) {
            if (buffer.status == MTLCommandBufferStatusError) {
                const char* description = buffer.error.localizedDescription.UTF8String;
                std::string message = description ? description : "Unknown Metal error";
                dispatch_async(dispatch_get_main_queue(), ^{
                    auto live = weak_app.lock();
                    guarded(live, [&] { report(live, "Metal presentation failed: " + message); });
                });
            }
        }];
        [command_buffer commit];
        if (sizing_transaction) {
            [command_buffer waitUntilScheduled];
            [drawable present];
        }
        if (app->capture_active) {
            [command_buffer waitUntilCompleted];
            [[MTLCaptureManager sharedCaptureManager] stopCapture];
            app->capture_active = false;
        }
        update_pause(surface);
    }
}
static void close_surface(std::shared_ptr<SurfaceState> surface, DialogResult result) {
    if (!surface || !surface->open || surface->closing) return;
    surface->closing = true;
    sync_secure_input(surface, true);
    const auto input = surface->text_input;
    if (input && input->unmark) guarded(surface, [&] { input->unmark(true); });
    if (input && input->deactivate) guarded(surface, [&] { input->deactivate(); });
    surface->text_input.reset(); surface->theme_changed = {};
    // Mark closed before calling user code; reentrant close/destruction is harmless.
    surface->open = false;
    surface->animating = false;
    [surface->display_link invalidate];
    surface->display_link = nil;
    auto children = surface->children;
    for (const auto& child : children) close_surface(child.lock());
    surface->children.clear();
    if (surface->modal && surface->window.sheetParent) {
        [surface->window.sheetParent endSheet:surface->window];
    }
    surface->modal = false;
    surface->window.delegate = nil;
    [surface->window close];
    surface->window = nil;
    surface->layer = nil;
    surface->view = nil;
    surface->delegate = nil;
    surface->menu = nil;
    surface->target = nil;
    surface->on_command = {};
    auto completion = std::exchange(surface->completion, {});
    auto closed = surface->callbacks.on_closed;
    surface->callbacks = {};
    surface->ui.reset();
    surface->accessibility.reset();
    guarded(surface, [&] { if (completion) completion(result); });
    guarded(surface, [&] { if (closed) closed(); });
    if (auto app = surface->application.lock()) {
        if (auto parent = surface->parent.lock(); parent && parent->open) activate_menu(parent);
        else if (!NSApp.keyWindow) NSApp.mainMenu = app->default_menu;
    }
}
static NSMenu* base_menu(const std::shared_ptr<ApplicationState>& app) {
    NSMenu* bar = [[NSMenu alloc] initWithTitle:@""];
    NSMenuItem* root = [[NSMenuItem alloc] initWithTitle:ns(app->options.name) action:nil keyEquivalent:@""];
    NSMenu* application_menu = [[NSMenu alloc] initWithTitle:ns(app->options.name)];
    [application_menu addItemWithTitle:[@"About " stringByAppendingString:ns(app->options.name)] action:@selector(orderFrontStandardAboutPanel:) keyEquivalent:@""];
    [application_menu addItem:NSMenuItem.separatorItem];
    NSMenuItem* new_window = [application_menu addItemWithTitle:@"New Window" action:@selector(newWindow:) keyEquivalent:@"n"];
    new_window.target = app->target;
    [application_menu addItem:NSMenuItem.separatorItem];
    [application_menu addItemWithTitle:[@"Hide " stringByAppendingString:ns(app->options.name)] action:@selector(hide:) keyEquivalent:@"h"];
    NSMenuItem* others = [application_menu addItemWithTitle:@"Hide Others" action:@selector(hideOtherApplications:) keyEquivalent:@"h"];
    others.keyEquivalentModifierMask = NSEventModifierFlagCommand | NSEventModifierFlagOption;
    [application_menu addItemWithTitle:@"Show All" action:@selector(unhideAllApplications:) keyEquivalent:@""];
    [application_menu addItem:NSMenuItem.separatorItem];
    [application_menu addItemWithTitle:[@"Quit " stringByAppendingString:ns(app->options.name)] action:@selector(terminate:) keyEquivalent:@"q"];
    root.submenu = application_menu;
    [bar addItem:root];
    return bar;
}
static NSMenu* build_menu(const std::vector<MenuItem>& items, SWMenuTarget* target) {
    NSMenu* menu = [[NSMenu alloc] initWithTitle:@""];
    menu.autoenablesItems = NO;
    for (const auto& item : items) {
        if (item.separator) { [menu addItem:NSMenuItem.separatorItem]; continue; }
        NSMenuItem* entry = [[NSMenuItem alloc] initWithTitle:ns(item.label) action:nil keyEquivalent:@""];
        entry.enabled = item.enabled;
        entry.state = item.checked ? NSControlStateValueOn : NSControlStateValueOff;
        if (!item.children.empty()) {
            entry.submenu = build_menu(item.children, target);
            entry.submenu.title = ns(item.label);
        } else {
            entry.action = @selector(command:);
            entry.target = target;
            entry.tag = item.command;
            entry.keyEquivalent = equivalent(item.shortcut.key);
            entry.keyEquivalentModifierMask = flags(item.shortcut.modifiers);
        }
        [menu addItem:entry];
    }
    return menu;
}
static std::expected<void, Error> validate_menu(std::span<const MenuItem> items) {
    std::unordered_set<Command> seen;
    const auto validate = [&](const auto& self, std::span<const MenuItem> nodes, bool top) -> bool {
        for (const auto& item : nodes) {
            if (top && (item.children.empty() || item.separator)) return false;
            if (item.separator) {
                if (item.command || !item.children.empty()) return false;
            } else if (item.label.empty()) return false;
            else if (!item.children.empty()) {
                if (item.command || !self(self, item.children, false)) return false;
            } else if (!item.command || !seen.insert(item.command).second) return false;
        }
        return true;
    };
    if (!validate(validate, items, true)) return std::unexpected(Error{"Menus require labelled top-level submenus and unique nonzero action commands"});
    return {};
}
static std::expected<std::shared_ptr<SurfaceState>, Error> create_surface(
    const std::shared_ptr<ApplicationState>& app, WindowOptions options,
    const std::shared_ptr<SurfaceState>& parent = {}) {
    main_thread();
    if (!app || !app->alive || (parent && !parent->open)) return std::unexpected(Error{"Application or parent window is closed"});
    for (const auto size : {options.size, options.minimum_size}) {
        if (!std::isfinite(size.width) || !std::isfinite(size.height) || size.width <= 0 || size.height <= 0 ||
            size.width > 32768 || size.height > 32768) return std::unexpected(Error{"Window dimensions must be finite and between 0 and 32768"});
    }
    if (options.size.width < options.minimum_size.width || options.size.height < options.minimum_size.height)
        return std::unexpected(Error{"Initial content size must be at least its minimum size"});
    auto surface = std::make_shared<SurfaceState>();
    surface->application = app;
    surface->parent = parent;
    surface->dialog = bool(parent);
    surface->title = options.title;
    auto style = NSWindowStyleMaskTitled | NSWindowStyleMaskClosable;
    if (!parent) style |= NSWindowStyleMaskMiniaturizable;
    if (options.resizable) style |= NSWindowStyleMaskResizable;
    NSWindow* window;
    if (options.floating && !parent) {
        NSPanel* panel = [[NSPanel alloc] initWithContentRect:NSMakeRect(0, 0, options.size.width, options.size.height)
            styleMask:style backing:NSBackingStoreBuffered defer:NO];
        panel.floatingPanel = YES;
        panel.hidesOnDeactivate = YES;
        panel.becomesKeyOnlyIfNeeded = NO;
        panel.level = NSFloatingWindowLevel;
        panel.collectionBehavior = NSWindowCollectionBehaviorFullScreenAuxiliary;
        window = panel;
    } else {
        window = [[NSWindow alloc] initWithContentRect:NSMakeRect(0, 0, options.size.width, options.size.height)
            styleMask:style backing:NSBackingStoreBuffered defer:NO];
    }
    if (!window) return std::unexpected(Error{"AppKit failed to create a window"});
    window.releasedWhenClosed = NO;
    window.title = ns(options.title);
    window.contentMinSize = NSMakeSize(options.minimum_size.width, options.minimum_size.height);
    window.tabbingMode = NSWindowTabbingModeDisallowed;
    window.acceptsMouseMovedEvents = YES;
    SWView* view = [[SWView alloc] initWithFrame:NSMakeRect(0, 0, options.size.width, options.size.height)];
    view->state = surface;
    view.wantsLayer = YES;
    CAMetalLayer* layer = [CAMetalLayer layer];
    layer.device = app->device;
    layer.pixelFormat = MTLPixelFormatBGRA8Unorm;
    layer.framebufferOnly = NO; // Skia may sample the drawable for blending; also permits test readback.
    layer.opaque = YES;
    CGColorSpaceRef colorspace = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
    layer.colorspace = colorspace;
    CGColorSpaceRelease(colorspace);
    view.layer = layer;
    window.contentView = view;
    SWWindowDelegate* delegate = [SWWindowDelegate new];
    delegate->state = surface;
    window.delegate = delegate;
    surface->window = window;
    surface->view = view;
    surface->layer = layer;
    surface->delegate = delegate;
    surface->target = [SWMenuTarget new];
    surface->target->state = surface;
    surface->target->application = app;
    surface->display_link = [view displayLinkWithTarget:view selector:@selector(tick:)];
    surface->display_link.paused = YES;
    [surface->display_link addToRunLoop:NSRunLoop.mainRunLoop forMode:NSRunLoopCommonModes];
    [window center];
    sync_size(surface);
    if (parent) parent->children.push_back(surface);
    std::erase_if(app->surfaces, [](const auto& entry) { return entry.expired(); });
    app->surfaces.push_back(surface);
    return surface;
}
} // namespace skia_window::detail

using namespace skia_window::detail;

static std::string utf8(NSString* value) {
    NSData* data = [value dataUsingEncoding:NSUTF8StringEncoding];
    return data ? std::string(static_cast<const char*>(data.bytes), data.length) : std::string{};
}
static std::optional<sw::TextInputRange> native_range(NSRange range) {
    return range.location == NSNotFound ? std::nullopt : std::optional(sw::TextInputRange{range.location, range.length});
}
static std::optional<sw::TextInputState> native_input(const std::shared_ptr<SurfaceState>& live) {
    std::optional<sw::TextInputState> result;
    guarded(live, [&] { if (live->open && live->text_input && live->text_input->state) result = live->text_input->state(); });
    return result;
}
namespace skia_window::detail {
static void sync_secure_input(const std::shared_ptr<SurfaceState>& live, bool force_off) {
    if (!live) return;
    bool desired = false;
    if (!force_off && live->open && !live->closing && !live->failed && NSApp.active &&
        live->window.keyWindow && live->window.visible && !live->window.miniaturized && interactive(live)) {
        const auto input = native_input(live);
        desired = input && input->secure && input->active && input->editable;
    }
    if (desired == live->secure_input_enabled) return;
    const auto status = desired ? EnableSecureEventInput() : DisableSecureEventInput();
    if (status == noErr)
        live->secure_input_enabled = desired;
    else {
        live->failed = true;
        report(live->application.lock(), "Unable to update secure keyboard input");
    }
}
} // namespace skia_window::detail
static bool native_text_command(const std::shared_ptr<SurfaceState>& live, const std::string& selector) {
    bool result = false;
    guarded(live, [&] { if (interactive(live) && live->text_input && live->text_input->command) result = live->text_input->command(selector); });
    sync_secure_input(live);
    return result;
}
static NSMenu* edit_menu() {
    NSMenu* menu = [[NSMenu alloc] initWithTitle:@"Edit"];
    auto item = [&](NSString* title, SEL action, NSString* key, bool shift = false) {
        NSMenuItem* entry = [[NSMenuItem alloc] initWithTitle:title action:action keyEquivalent:key];
        entry.keyEquivalentModifierMask = NSEventModifierFlagCommand | (shift ? NSEventModifierFlagShift : 0);
        [menu addItem:entry];
    };
    item(@"Undo", @selector(undo:), @"z"); item(@"Redo", @selector(redo:), @"z", true);
    [menu addItem:NSMenuItem.separatorItem];
    item(@"Cut", @selector(cut:), @"x"); item(@"Copy", @selector(copy:), @"c"); item(@"Paste", @selector(paste:), @"v");
    item(@"Delete", @selector(delete:), @""); [menu addItem:NSMenuItem.separatorItem];
    item(@"Select All", @selector(selectAll:), @"a"); return menu;
}
static void install_edit_menu(const std::shared_ptr<SurfaceState>& live) {
    if (!live->text_input) return;
    if (!live->menu) live->menu = base_menu(live->application.lock());
    for (NSMenuItem* item in live->menu.itemArray) if ([item.title isEqualToString:@"Edit"]) return;
    NSMenuItem* entry = [[NSMenuItem alloc] initWithTitle:@"Edit" action:nil keyEquivalent:@""];
    entry.submenu = edit_menu(); [live->menu insertItem:entry atIndex:MIN(2, live->menu.numberOfItems)];
    if (live->window.keyWindow) activate_menu(live);
}
static std::optional<sw::ui::ElementInfo> ui_element(const std::shared_ptr<SurfaceState>& surface, const std::string& id) {
    std::optional<sw::ui::ElementInfo> result;
    guarded(surface, [&] {
        auto host = surface->ui;
        if (!host || !surface->open) return;
        host->layout(sw::detail::content_size(surface));
        for (auto& element : host->elements()) if (element.id == id) { element.enabled = element.enabled && interactive(surface); result = std::move(element); break; }
    });
    return result;
}
@implementation SWAccessibilityElement
- (BOOL)isAccessibilityElement { return YES; }
- (id)accessibilityParent { auto live = state.lock(); return live ? live->view : nil; }
- (NSString*)accessibilityIdentifier { return ns(element_id); }
- (NSString*)accessibilityRole {
    auto info = ui_element(state.lock(), element_id);
    if (!info) return NSAccessibilityUnknownRole;
    switch (info->role) {
        case sw::ui::Role::text: return NSAccessibilityStaticTextRole;
        case sw::ui::Role::button: return NSAccessibilityButtonRole;
        case sw::ui::Role::checkbox: return NSAccessibilityCheckBoxRole;
        case sw::ui::Role::slider: return NSAccessibilitySliderRole;
        case sw::ui::Role::image: return NSAccessibilityImageRole;
        case sw::ui::Role::canvas: return NSAccessibilityGroupRole;
    }
    return NSAccessibilityUnknownRole;
}
- (NSString*)accessibilityLabel {
    auto info = ui_element(state.lock(), element_id);
    return info ? ns(info->label) : @"";
}
- (id)accessibilityValue {
    auto info = ui_element(state.lock(), element_id);
    if (!info) return nil;
    if (info->role == sw::ui::Role::checkbox) return @(info->checked);
    if (info->role == sw::ui::Role::slider) return @(info->value);
    if (info->role == sw::ui::Role::text) return ns(info->label);
    return nil;
}
- (void)setAccessibilityValue:(id)value {
    if (![value respondsToSelector:@selector(doubleValue)]) return;
    auto live = state.lock();
    guarded(live, [&] { auto host = live->ui; if (host && interactive(live)) host->set_value(element_id,[value doubleValue]); });
    NSAccessibilityPostNotification(self, NSAccessibilityValueChangedNotification);
}
- (id)accessibilityMinValue { auto info = ui_element(state.lock(), element_id); return info ? @(info->minimum) : nil; }
- (id)accessibilityMaxValue { auto info = ui_element(state.lock(), element_id); return info ? @(info->maximum) : nil; }
- (BOOL)isAccessibilityEnabled { auto info = ui_element(state.lock(), element_id); return info && info->enabled; }
- (BOOL)isAccessibilityFocused { auto info = ui_element(state.lock(), element_id); return info && info->focused; }
- (void)setAccessibilityFocused:(BOOL)value {
    if (!value) return;
    auto live = state.lock();
    guarded(live, [&] {
        auto host = live->ui;
        if (host && interactive(live) && host->focus(element_id)) [live->window makeFirstResponder:live->view];
    });
}
- (NSRect)accessibilityFrame {
    auto live = state.lock(); auto info = ui_element(live, element_id);
    if (!live || !info) return NSZeroRect;
    auto r = info->bounds;
    auto clip = info->clip;
    auto visible = NSIntersectionRect(NSMakeRect(r.x,r.y,r.width,r.height), NSMakeRect(clip.x,clip.y,clip.width,clip.height));
    if (NSIsEmptyRect(visible)) return NSZeroRect;
    return [live->window convertRectToScreen:[live->view convertRect:visible toView:nil]];
}
- (BOOL)accessibilityPerformPress {
    auto live = state.lock(); bool result = false;
    guarded(live, [&] { auto host = live->ui; if (host && interactive(live)) { host->focus(element_id); result = host->activate(element_id); } });
    if (result) NSAccessibilityPostNotification(self, NSAccessibilityValueChangedNotification);
    return result;
}
- (BOOL)accessibilityPerformIncrement {
    auto live = state.lock(); bool result = false;
    guarded(live, [&] { auto host = live->ui; if (host && interactive(live)) result = host->adjust(element_id,1); });
    if (result) NSAccessibilityPostNotification(self, NSAccessibilityValueChangedNotification);
    return result;
}
- (BOOL)accessibilityPerformDecrement {
    auto live = state.lock(); bool result = false;
    guarded(live, [&] { auto host = live->ui; if (host && interactive(live)) result = host->adjust(element_id,-1); });
    if (result) NSAccessibilityPostNotification(self, NSAccessibilityValueChangedNotification);
    return result;
}
@end

static std::optional<sw::AccessibilityElement> external_element(const std::shared_ptr<SurfaceState>& live,
                                                                const std::string& id) {
    std::optional<sw::AccessibilityElement> result;
    guarded(live, [&] {
        auto provider = live->accessibility;
        if (!live->open || !provider)
            return;
        if (provider->prepare)
            provider->prepare(sw::detail::content_size(live));
        if (!live->open || live->accessibility != provider)
            return;
        if (provider->logical_element)
            result = provider->logical_element(id);
        if (!result && provider->elements)
            for (auto& info : provider->elements(sw::detail::content_size(live)))
                if (info.id == id) {
                    result = std::move(info);
                    break;
                }
        if (!live->open || live->accessibility != provider)
            result.reset();
        if (result)
            result->enabled = result->enabled && interactive(live);
    });
    return result;
}
static id external_object(const std::shared_ptr<SurfaceState>& live, const std::string& id) {
    if (live && live->open && live->view)
        if (auto cached = live->view->accessibility_nodes[ns(id)])
            return cached;
    auto value = [SWExternalAccessibilityElement new];
    value->state = live;
    value->element_id = id;
    return value;
}
@implementation SWLogicalAccessibilityObjects
- (NSUInteger)count {
    return ids.size();
}
- (id)objectAtIndex:(NSUInteger)index {
    if (index >= ids.size())
        [NSException raise:NSRangeException format:@"Logical object index exceeds selection"];
    return external_object(state.lock(), ids.at(index));
}
- (id)copyWithZone:(NSZone*)zone {
    return self;
}
@end
@implementation SWLogicalAccessibilityRows
- (NSUInteger)count {
    const auto root = external_element(state.lock(), list_id);
    if (!root)
        return 0;
    if (cell_mode == 1 || cell_mode == 4)
        return root->column_count.value_or(0);
    if (cell_mode == 2)
        return root->item_count.value_or(0);
    if (cell_mode == 3)
        return root->selected_items.size() * root->column_count.value_or(0);
    if (children_only)
        return root->disclosed_count;
    return root->item_count ? selected_only ? root->selected_items.size() : *root->item_count : 0;
}
- (id)objectAtIndex:(NSUInteger)index {
    if (index >= [self count])
        [NSException raise:NSRangeException format:@"Logical collection index exceeds count"];
    auto live = state.lock();
    auto root = external_element(live, list_id);
    std::optional<sw::AccessibilityElement> info;
    guarded(live, [&] {
        auto p = live->accessibility;
        if (!p || !root)
            return;
        if (cell_mode == 1 && p->cell_at)
            info = p->cell_at(list_id, index);
        else if (cell_mode == 2 && p->column_cell_at)
            info = p->column_cell_at(list_id, index);
        else if (cell_mode == 4 && p->column_at)
            info = p->column_at(list_id, index);
        else if (cell_mode == 3 && p->item_at && p->cell_at && root->column_count && *root->column_count) {
            const auto c = *root->column_count;
            if (auto row = p->item_at(list_id, root->selected_items.at(index / c)))
                info = p->cell_at(row->id, index % c);
        } else if (children_only && p->child_at)
            info = p->child_at(list_id, index);
        else if (p->item_at)
            info = p->item_at(list_id, selected_only ? root->selected_items.at(index) : index);
    });
    if (!info)
        [NSException raise:NSRangeException format:@"Logical element is no longer available"];
    return external_object(live, info->id);
}
- (id)copyWithZone:(NSZone*)zone {
    return self;
}
@end
@implementation SWExternalAccessibilityElement
- (BOOL)isAccessibilityElement { return YES; }
- (NSString*)accessibilityIdentifier { return ns(element_id); }
- (id)accessibilityParent {
    auto live = state.lock(); auto info = external_element(live, element_id);
    if (live && info && !info->group.empty()) {
        auto parent = live->view->accessibility_nodes[ns(info->group)];
        if (parent) return parent;
        if (external_element(live, info->group)) {
            auto value = [SWExternalAccessibilityElement new]; value->state = live; value->element_id = info->group;
            return value;
        }
    }
    return live ? live->view : nil;
}
- (NSArray*)accessibilityChildren {
    auto live=state.lock();if(!live||!live->accessibility)return @[];
    auto info=external_element(live,element_id);
    if(info&&info->role==sw::AccessibilityRole::column&&live->accessibility->column_cell_at){
        auto cells=[SWLogicalAccessibilityRows new];cells->state=live;cells->list_id=element_id;cells->cell_mode=2;return cells;
    }
    if(info&&info->role==sw::AccessibilityRole::list_item&&info->column_count&&live->accessibility->cell_at){
        auto cells=[SWLogicalAccessibilityRows new];cells->state=live;cells->list_id=element_id;cells->cell_mode=1;return cells;
    }
    if(info&&info->item_count&&live->accessibility->item_at){
        auto rows=[SWLogicalAccessibilityRows new];rows->state=live;rows->list_id=element_id;return rows;
    }
    NSMutableArray* result = [NSMutableArray array];
    guarded(live, [&] {
        for (const auto& info : live->accessibility->elements(sw::detail::content_size(live)))
            if (info.group == element_id) {
                auto child = live->view->accessibility_nodes[ns(info.id)];
                if (!child) {
                    auto value = [SWExternalAccessibilityElement new]; value->state = live; value->element_id = info.id;
                    child = value;
                }
                if (child) [result addObject:child];
            }
    });
    return result;
}
- (NSInteger)accessibilityColumnCount {
    auto info = external_element(state.lock(), element_id);
    return info && info->column_count ? static_cast<NSInteger>(*info->column_count) : 0;
}
- (NSArray*)accessibilityColumns {
    auto live = state.lock();
    auto info = external_element(live, element_id);
    if (!info || info->role != sw::AccessibilityRole::table || !info->column_count)
        return nil;
    auto values = [SWLogicalAccessibilityRows new];
    values->state = live;
    values->list_id = element_id;
    values->cell_mode = 4;
    return values;
}
- (NSArray*)accessibilityVisibleColumns {
    auto all = [self accessibilityColumns];
    if (!all)
        return nil;
    NSMutableArray* result = [NSMutableArray array];
    for (id col in all)
        if (!NSEqualRects([col accessibilityFrame], NSZeroRect))
            [result addObject:col];
    return result;
}
- (NSArray*)accessibilityColumnHeaderUIElements {
    auto all = [self accessibilityColumns];
    if (!all)
        return nil;
    NSMutableArray* result = [NSMutableArray array];
    for (id col in all)
        if (id header = [col accessibilityHeader])
            [result addObject:header];
    return result;
}
- (NSArray*)accessibilitySelectedColumns {
    const auto info=external_element(state.lock(),element_id);return info&&info->role==sw::AccessibilityRole::table?@[]:nil;
}
- (id)accessibilityHeader {
    auto live = state.lock();
    auto info = external_element(live, element_id);
    return info && !info->header_id.empty() ? external_object(live, info->header_id) : nil;
}
- (NSAccessibilitySortDirection)accessibilitySortDirection {
    auto info = external_element(state.lock(), element_id);
    return !info || !info->sort_ascending ? NSAccessibilitySortDirectionUnknown
           : *info->sort_ascending        ? NSAccessibilitySortDirectionAscending
                                          : NSAccessibilitySortDirectionDescending;
}
- (NSRange)accessibilityRowIndexRange {
    auto info = external_element(state.lock(), element_id);
    return info && info->item_index ? NSMakeRange(*info->item_index, 1) : NSMakeRange(NSNotFound, 0);
}
- (NSRange)accessibilityColumnIndexRange {
    auto info = external_element(state.lock(), element_id);
    return info && info->column_index ? NSMakeRange(*info->column_index, 1) : NSMakeRange(NSNotFound, 0);
}
- (id)accessibilityCellForColumn:(NSInteger)column row:(NSInteger)row {
    if (column < 0 || row < 0)
        return nil;
    auto live = state.lock();
    auto info = external_element(live, element_id);
    if (!info || info->role != sw::AccessibilityRole::table || !info->item_count || !info->column_count ||
        std::size_t(row) >= *info->item_count || std::size_t(column) >= *info->column_count)
        return nil;
    std::optional<sw::AccessibilityElement> cell;
    guarded(live, [&] {
        auto p = live->accessibility;
        if (p && p->item_at && p->cell_at)
            if (auto item = p->item_at(element_id, row))
                cell = p->cell_at(item->id, column);
    });
    return cell ? external_object(live, cell->id) : nil;
}
- (NSArray*)accessibilitySelectedCells {
    auto live = state.lock();
    auto info = external_element(live, element_id);
    if (!info || info->role != sw::AccessibilityRole::table)
        return nil;
    if (!info->selected_cells.empty()) {
        auto values = [SWLogicalAccessibilityObjects new];
        values->state = live;
        values->ids = info->selected_cells;
        return values;
    }
    auto values = [SWLogicalAccessibilityRows new];
    values->state = live;
    values->list_id = element_id;
    values->cell_mode = 3;
    return values;
}
- (void)setAccessibilitySelectedCells:(NSArray*)values {
    auto live = state.lock();
    std::vector<std::string> ids;
    for (id value in values) {
        if (![value respondsToSelector:@selector(accessibilityIdentifier)])
            return;
        auto identifier = [value accessibilityIdentifier];
        if (!identifier)
            return;
        ids.push_back(utf8(identifier));
    }
    guarded(live, [&] {
        auto p = live->accessibility;
        if (interactive(live) && p && p->set_selection && p->set_selection(element_id, std::move(ids))) {
            NSAccessibilityPostNotification(self, NSAccessibilitySelectedCellsChangedNotification);
            NSAccessibilityPostNotification(self, NSAccessibilitySelectedRowsChangedNotification);
        }
    });
}
- (void)setAccessibilitySelectedRows:(NSArray*)values {
    auto live = state.lock();
    std::vector<std::string> ids;
    for (id value in values) {
        if (![value respondsToSelector:@selector(accessibilityIdentifier)])
            return;
        auto identifier = [value accessibilityIdentifier];
        if (!identifier)
            return;
        ids.push_back(utf8(identifier));
    }
    guarded(live, [&] {
        auto p = live->accessibility;
        if (interactive(live) && p && p->set_selection && p->set_selection(element_id, std::move(ids))) {
            NSAccessibilityPostNotification(self, NSAccessibilitySelectedCellsChangedNotification);
            NSAccessibilityPostNotification(self, NSAccessibilitySelectedRowsChangedNotification);
        }
    });
}
- (NSArray*)accessibilityVisibleCells {
    auto live = state.lock();
    auto root = external_element(live, element_id);
    if (!root || root->role != sw::AccessibilityRole::table)
        return nil;
    NSMutableArray* cells = [NSMutableArray array];
    guarded(live, [&] {
        for (const auto& info : live->accessibility->elements(sw::detail::content_size(live)))
            if (info.role == sw::AccessibilityRole::cell && info.table_id == element_id &&
                info.clip_size.width > 0 && info.clip_size.height > 0)
                [cells addObject:external_object(live, info.id)];
    });
    return cells;
}
- (BOOL)accessibilityPerformConfirm {
    auto live = state.lock();
    auto info = external_element(live, element_id);
    bool result = false;
    if (!info || info->role != sw::AccessibilityRole::cell || !info->enabled || info->read_only)
        return NO;
    guarded(live, [&] {
        auto p = live->accessibility;
        if (interactive(live) && p && p->begin_edit)
            result = p->begin_edit(element_id);
        if (result)
            [live->window makeFirstResponder:live->view];
    });
    sync_secure_input(live);
    return result;
}
- (NSInteger)accessibilityRowCount {
    auto info = external_element(state.lock(), element_id);
    return info && info->item_count ? static_cast<NSInteger>(*info->item_count) : 0;
}
- (NSInteger)accessibilityIndex {
    auto info = external_element(state.lock(), element_id);
    return !info?NSNotFound:info->role==sw::AccessibilityRole::column||info->role==sw::AccessibilityRole::column_header?info->column_index?static_cast<NSInteger>(*info->column_index):NSNotFound:info->item_index?static_cast<NSInteger>(*info->item_index):NSNotFound;
}
- (NSArray*)accessibilityRows {
    auto info = external_element(state.lock(), element_id);
    return info && info->item_count ? [self accessibilityChildren] : nil;
}
- (NSArray*)accessibilitySelectedRows {
    auto live = state.lock(); auto info = external_element(live, element_id);
    if (!info || !info->item_count || !live->accessibility->item_at) return nil;
    auto rows = [SWLogicalAccessibilityRows new]; rows->state = live; rows->list_id = element_id; rows->selected_only = true;
    return rows;
}
- (NSArray*)accessibilityVisibleRows {
    auto live = state.lock(); auto info = external_element(live, element_id);
    if (!info || !info->item_count) return nil;
    NSMutableArray* rows = [NSMutableArray array];
    guarded(live, [&] {
        for (const auto& child : live->accessibility->elements(sw::detail::content_size(live)))
            if (child.group == element_id && child.item_index && child.clip_size.width > 0 && child.clip_size.height > 0) {
                auto row = live->view->accessibility_nodes[ns(child.id)];
                if (!row) { auto value = [SWExternalAccessibilityElement new]; value->state = live; value->element_id = child.id; row = value; }
                [rows addObject:row];
            }
    });
    return rows;
}
- (NSString*)accessibilityRole {
    auto info = external_element(state.lock(), element_id); if (!info) return NSAccessibilityUnknownRole;
    switch (info->role) {
    case sw::AccessibilityRole::text: return NSAccessibilityStaticTextRole;
    case sw::AccessibilityRole::button: return NSAccessibilityButtonRole;
    case sw::AccessibilityRole::checkbox: return NSAccessibilityCheckBoxRole;
    case sw::AccessibilityRole::radio: return NSAccessibilityRadioButtonRole;
    case sw::AccessibilityRole::slider: return NSAccessibilitySliderRole;
    case sw::AccessibilityRole::splitter: return NSAccessibilitySplitterRole;
    case sw::AccessibilityRole::tab: return NSAccessibilityRadioButtonRole;
    case sw::AccessibilityRole::tab_group: return NSAccessibilityTabGroupRole;
    case sw::AccessibilityRole::stepper: return NSAccessibilityIncrementorRole;
    case sw::AccessibilityRole::switch_control: return NSAccessibilityCheckBoxRole;
    case sw::AccessibilityRole::progress: return NSAccessibilityProgressIndicatorRole;
    case sw::AccessibilityRole::group: case sw::AccessibilityRole::alert: return NSAccessibilityGroupRole;
    case sw::AccessibilityRole::menu_button: return NSAccessibilityMenuButtonRole;
    case sw::AccessibilityRole::selection: return NSAccessibilityPopUpButtonRole;
    case sw::AccessibilityRole::menu_item: return NSAccessibilityMenuItemRole;
    case sw::AccessibilityRole::menu: return NSAccessibilityMenuRole;
    case sw::AccessibilityRole::image: return NSAccessibilityImageRole;
    case sw::AccessibilityRole::list_box: return NSAccessibilityListRole;
    case sw::AccessibilityRole::table: return NSAccessibilityTableRole;
    case sw::AccessibilityRole::column_header: return info->sortable?NSAccessibilityButtonRole:NSAccessibilityStaticTextRole;
    case sw::AccessibilityRole::column: return NSAccessibilityColumnRole;
    case sw::AccessibilityRole::cell: return NSAccessibilityCellRole;
    case sw::AccessibilityRole::tree_view: return NSAccessibilityOutlineRole;
    case sw::AccessibilityRole::list_item: return NSAccessibilityRowRole;
    case sw::AccessibilityRole::text_field: return info->multiline ? NSAccessibilityTextAreaRole : NSAccessibilityTextFieldRole;
    case sw::AccessibilityRole::combo_box: return NSAccessibilityComboBoxRole;
    }
    return NSAccessibilityUnknownRole;
}
- (NSString*)accessibilityLabel { auto info=external_element(state.lock(),element_id);return info?ns(info->label):@""; }
- (NSString*)accessibilityHelp { auto info=external_element(state.lock(),element_id);return info?ns(info->help):@""; }
- (NSString*)accessibilitySubrole {
    auto info = external_element(state.lock(), element_id);
    if(info&&info->role==sw::AccessibilityRole::column_header&&info->sortable)return NSAccessibilitySortButtonSubrole;
    if (info && info->role == sw::AccessibilityRole::alert) return @"AXDialog";
    if (info && info->secure) return NSAccessibilitySecureTextFieldSubrole;
    if (info && info->search) return NSAccessibilitySearchFieldSubrole;
    return info && info->role == sw::AccessibilityRole::switch_control ? NSAccessibilitySwitchSubrole : nil;
}
- (id)accessibilityDefaultButton {
    auto live = state.lock();
    const auto info = external_element(live, element_id);
    if (!info || info->role != sw::AccessibilityRole::alert || !live->accessibility->elements)
        return nil;
    for (const auto& child : live->accessibility->elements(sw::detail::content_size(live)))
        if (child.group == element_id && child.default_action)
            return external_object(live, child.id);
    return nil;
}
- (id)accessibilityCancelButton {
    auto live = state.lock();
    const auto info = external_element(live, element_id);
    if (!info || info->role != sw::AccessibilityRole::alert || !live->accessibility->elements)
        return nil;
    for (const auto& child : live->accessibility->elements(sw::detail::content_size(live)))
        if (child.group == element_id && child.cancel_action)
            return external_object(live, child.id);
    return nil;
}
- (NSString*)accessibilityValueDescription {
    auto info = external_element(state.lock(), element_id);
    return info && info->role == sw::AccessibilityRole::stepper ? ns(info->value_text) : nil;
}
- (id)accessibilityValue {
    auto info=external_element(state.lock(),element_id);if(!info)return nil;
    if(info->role==sw::AccessibilityRole::checkbox||info->role==sw::AccessibilityRole::radio||info->role==sw::AccessibilityRole::tab||info->role==sw::AccessibilityRole::switch_control)return @(info->mixed?2:info->selected?1:0);
    if(info->role==sw::AccessibilityRole::slider||info->role==sw::AccessibilityRole::progress||info->role==sw::AccessibilityRole::splitter||info->role==sw::AccessibilityRole::stepper)return @(info->value);
    return ns(info->value_text);
}
- (BOOL)isAccessibilityEnabled {
    auto live = state.lock();
    auto info = external_element(live, element_id);
    return info && info->enabled && live && interactive(live);
}
- (NSAccessibilityOrientation)accessibilityOrientation {
    auto info = external_element(state.lock(), element_id);
    if (!info || (info->role != sw::AccessibilityRole::splitter && info->role != sw::AccessibilityRole::slider))
        return NSAccessibilityOrientationUnknown;
    return info->vertical ? NSAccessibilityOrientationVertical : NSAccessibilityOrientationHorizontal;
}
- (BOOL)isAccessibilitySelected { auto info=external_element(state.lock(),element_id);return info&&info->selected; }
- (void)setAccessibilitySelected:(BOOL)value {
    auto live=state.lock();auto info=external_element(live,element_id);
    if(info&&info->selected!=bool(value))guarded(live,[&]{
        auto p=live->accessibility;if(!interactive(live)||!p)return;
        bool changed=p->set_selected?p->set_selected(element_id,value):p->activate?p->activate(element_id):false;
        if(changed){const auto owner=!info->table_id.empty()?info->table_id:info->group;
            id target=owner.empty()?self:external_object(live,owner);
            NSAccessibilityPostNotification(target,NSAccessibilitySelectedRowsChangedNotification);
            if(!info->table_id.empty())NSAccessibilityPostNotification(target,NSAccessibilitySelectedCellsChangedNotification);
        }
    });
}
- (BOOL)isAccessibilityFocused { auto info=external_element(state.lock(),element_id);return info&&info->focused; }
- (BOOL)isAccessibilityExpanded { auto info=external_element(state.lock(),element_id);return info&&info->expanded; }
- (void)setAccessibilityExpanded:(BOOL)value {
    auto live = state.lock(); auto info = external_element(live, element_id);
    guarded(live, [&] {
        if (info && info->expandable && info->enabled && live->accessibility && live->accessibility->set_expanded)
            live->accessibility->set_expanded(element_id, value);
    });
}
- (BOOL)isAccessibilityDisclosed { return [self isAccessibilityExpanded]; }
- (void)setAccessibilityDisclosed:(BOOL)value { [self setAccessibilityExpanded:value]; }
- (NSInteger)accessibilityDisclosureLevel {
    auto info = external_element(state.lock(), element_id);
    return info && info->tree_level ? static_cast<NSInteger>(*info->tree_level) : 0;
}
- (id)accessibilityDisclosedByRow {
    auto live = state.lock(); auto info = external_element(live, element_id);
    if (!live || !info || info->tree_parent.empty()) return nil;
    auto value = [SWExternalAccessibilityElement new]; value->state = live; value->element_id = info->tree_parent;
    return value;
}
- (NSArray*)accessibilityDisclosedRows {
    auto live = state.lock(); auto info = external_element(live, element_id);
    if (!info || !info->tree_level || !live->accessibility->child_at) return nil;
    auto rows = [SWLogicalAccessibilityRows new]; rows->state = live; rows->list_id = element_id; rows->children_only = true;
    return rows;
}
- (id)accessibilityMinValue { auto info=external_element(state.lock(),element_id);return info?@(info->minimum):nil; }
- (id)accessibilityMaxValue { auto info=external_element(state.lock(),element_id);return info?@(info->maximum):nil; }
- (NSRect)accessibilityFrame {
    auto live=state.lock();auto info=external_element(live,element_id);if(!live||!info)return NSZeroRect;
    auto r=NSIntersectionRect(NSMakeRect(info->origin.x,info->origin.y,info->size.width,info->size.height),NSMakeRect(info->clip_origin.x,info->clip_origin.y,info->clip_size.width,info->clip_size.height));
    return NSIsEmptyRect(r)?NSZeroRect:[live->window convertRectToScreen:[live->view convertRect:r toView:nil]];
}
- (void)setAccessibilityFocused:(BOOL)value {
    auto live=state.lock();if(!value)return;
    guarded(live,[&]{auto p=live->accessibility;if(interactive(live)&&p&&p->focus&&p->focus(element_id))[live->window makeFirstResponder:live->view];});
    sync_secure_input(live);
}
- (NSRange)accessibilitySelectedTextRange {
    auto info = external_element(state.lock(), element_id);
    return info && !info->secure ? NSMakeRange(info->text_selection_start, info->text_selection_length) : NSMakeRange(NSNotFound, 0);
}
- (void)setAccessibilitySelectedTextRange:(NSRange)range {
    auto live = state.lock(); guarded(live, [&] {
        auto info = external_element(live, element_id);
        if (info && info->secure) return;
        if (live->accessibility && live->accessibility->select_text)
            live->accessibility->select_text(element_id, range.location, range.length);
    });
}
- (NSString*)accessibilitySelectedText {
    auto info = external_element(state.lock(), element_id); if (!info) return @"";
    if (info->secure) return @"";
    auto value = ns(info->value_text); const auto start = MIN(value.length, info->text_selection_start);
    return [value substringWithRange:NSMakeRange(start, MIN(info->text_selection_length, value.length - start))];
}
- (void)setAccessibilityValue:(id)value {
    auto live = state.lock(); auto info = external_element(live, element_id);
    if (info && (info->role == sw::AccessibilityRole::text_field || info->role == sw::AccessibilityRole::combo_box || (info->role==sw::AccessibilityRole::cell&&!info->read_only))) {
        if ([value isKindOfClass:NSString.class]) guarded(live, [&] {
            if (live->accessibility && live->accessibility->set_text) live->accessibility->set_text(element_id, utf8(value));
        });
        return;
    }
    if(![value respondsToSelector:@selector(doubleValue)])return;
    guarded(live,[&]{auto p=live->accessibility;if(interactive(live)&&p&&p->set_value)p->set_value(element_id,[value doubleValue]);});
    NSAccessibilityPostNotification(self,NSAccessibilityValueChangedNotification);
}
- (BOOL)accessibilityPerformPress {
    auto live=state.lock();bool result=false;
    guarded(live,[&]{auto p=live->accessibility;if(interactive(live)&&p&&p->activate)result=p->activate(element_id);});
    sync_secure_input(live);
    if(result)NSAccessibilityPostNotification(self,NSAccessibilityValueChangedNotification);return result;
}
- (BOOL)accessibilityPerformIncrement {
    auto live=state.lock();bool result=false;
    guarded(live,[&]{auto p=live->accessibility;if(interactive(live)&&p&&p->adjust)result=p->adjust(element_id,1);});return result;
}
- (BOOL)accessibilityPerformDecrement {
    auto live=state.lock();bool result=false;
    guarded(live,[&]{auto p=live->accessibility;if(interactive(live)&&p&&p->adjust)result=p->adjust(element_id,-1);});return result;
}
@end

static NSCursor* surface_cursor(sw::Cursor cursor) {
    switch (cursor) {
    case sw::Cursor::text: return [NSCursor IBeamCursor];
    case sw::Cursor::resize_horizontal: return [NSCursor resizeLeftRightCursor];
    case sw::Cursor::resize_vertical: return [NSCursor resizeUpDownCursor];
    default: return [NSCursor arrowCursor];
    }
}
@implementation SWView
- (void)resetCursorRects {
    [super resetCursorRects];
    [self addCursorRect:self.bounds cursor:surface_cursor(pointer_cursor)];
}
- (BOOL)isAccessibilityElement { auto live = state.lock(); return live && (live->ui || live->accessibility); }
- (NSString*)accessibilityRole { auto live = state.lock(); return live && (live->ui || live->accessibility) ? NSAccessibilityGroupRole : [super accessibilityRole]; }
- (NSString*)accessibilityLabel { auto live = state.lock(); return live && (live->ui || live->accessibility) ? @"Content" : [super accessibilityLabel]; }
- (NSArray*)accessibilityChildren {
    auto live = state.lock();
    if (live && live->accessibility) {
        NSMutableArray* children=[NSMutableArray array];
        guarded(live,[&]{
            auto provider=live->accessibility;if(!provider->elements)return;
            if(!accessibility_nodes)accessibility_nodes=[NSMutableDictionary dictionary];
            NSMutableSet<NSString*>* current=[NSMutableSet set];
            for(const auto& info:provider->elements(sw::detail::content_size(live))) {
                NSString* key=ns(info.id);[current addObject:key];
                auto element=(SWExternalAccessibilityElement*)accessibility_nodes[key];
                if(!element){element=[SWExternalAccessibilityElement new];element->state=live;element->element_id=info.id;accessibility_nodes[key]=element;}
                if(info.group.empty())[children addObject:element];
            }
            for(NSString* key in [accessibility_nodes.allKeys copy])if(![current containsObject:key])[accessibility_nodes removeObjectForKey:key];
        });
        return children;
    }
    if (!live || !live->ui) return [super accessibilityChildren];
    NSMutableArray* children = [NSMutableArray array];
    guarded(live, [&] {
        auto host = live->ui;
        host->layout(sw::detail::content_size(live));
        if (!accessibility_nodes) accessibility_nodes = [NSMutableDictionary dictionary];
        NSMutableSet<NSString*>* current = [NSMutableSet set];
        for (const auto& info : host->elements()) {
            NSString* key = ns(info.id);
            [current addObject:key];
            SWAccessibilityElement* element = (SWAccessibilityElement*)accessibility_nodes[key];
            if (!element) { element = [SWAccessibilityElement new]; element->state = live; element->element_id = info.id; accessibility_nodes[key] = element; }
            [children addObject:element];
        }
        for (NSString* key in [accessibility_nodes.allKeys copy]) if (![current containsObject:key]) [accessibility_nodes removeObjectForKey:key];
    });
    return children;
}
- (id)accessibilityHitTest:(NSPoint)point {
    auto live=state.lock();
    if(live && live->accessibility) {
        [self accessibilityChildren];
        auto infos=live->accessibility->elements(sw::detail::content_size(live));
        std::stable_sort(infos.begin(), infos.end(), [](const auto& a, const auto& b) {
            return a.paint_order < b.paint_order;
        });
        for(auto it=infos.rbegin();it!=infos.rend();++it) {
            NSAccessibilityElement* element=accessibility_nodes[ns(it->id)];
            if(element && NSPointInRect(point,element.accessibilityFrame))return element;
        }
    }
    for (NSAccessibilityElement* element in [[self accessibilityChildren] reverseObjectEnumerator]) {
        if (NSPointInRect(point, element.accessibilityFrame)) return element;
    }
    return [super accessibilityHitTest:point];
}
- (id)accessibilityFocusedUIElement {
    [self accessibilityChildren];
    for (NSAccessibilityElement* element in accessibility_nodes.allValues) if (element.accessibilityFocused) return element;
    return self;
}
- (BOOL)isFlipped { return YES; }
- (BOOL)acceptsFirstResponder { return YES; }
- (BOOL)acceptsFirstMouse:(NSEvent*)event { return YES; }
- (NSRange)selectedRange {
    auto input = native_input(state.lock());
    return input ? NSMakeRange(input->selection.start, input->selection.length) : NSMakeRange(NSNotFound, 0);
}
- (NSRange)markedRange {
    auto input = native_input(state.lock());
    return input && input->marked ? NSMakeRange(input->marked->start, input->marked->length) : NSMakeRange(NSNotFound, 0);
}
- (BOOL)hasMarkedText { return [self markedRange].location != NSNotFound; }
- (void)insertText:(id)value replacementRange:(NSRange)range {
    auto live = state.lock();
    guarded(live, [&] {
        if (!live->text_input || !live->text_input->insert || !interactive(live)) return;
        NSString* text = [value isKindOfClass:NSAttributedString.class] ? [value string] : value;
        live->text_input->insert(utf8(text), native_range(range));
        sync_secure_input(live);
        [self.inputContext invalidateCharacterCoordinates];
    });
}
- (void)setMarkedText:(id)value selectedRange:(NSRange)selected replacementRange:(NSRange)replacement {
    auto live = state.lock();
    guarded(live, [&] {
        if (!live->text_input || !live->text_input->mark || !interactive(live)) return;
        NSString* text = [value isKindOfClass:NSAttributedString.class] ? [value string] : value;
        live->text_input->mark(utf8(text), {selected.location, selected.length}, native_range(replacement));
        sync_secure_input(live);
        [self.inputContext invalidateCharacterCoordinates];
    });
}
- (void)unmarkText {
    auto live = state.lock(); guarded(live, [&] {
        if (live->text_input && live->text_input->unmark) live->text_input->unmark(true);
        sync_secure_input(live);
    });
}
- (NSArray<NSAttributedStringKey>*)validAttributesForMarkedText { return @[NSUnderlineStyleAttributeName]; }
- (NSAttributedString*)attributedSubstringForProposedRange:(NSRange)range actualRange:(NSRangePointer)actual {
    auto input = native_input(state.lock());
    if (!input || input->secure || range.location == NSNotFound) {
        if (actual) *actual = NSMakeRange(NSNotFound, 0);
        return nil;
    }
    NSString* text = ns(input->text);
    range.location = MIN(range.location, text.length); range.length = MIN(range.length, text.length - range.location);
    if (actual) *actual = range;
    return [[NSAttributedString alloc] initWithString:[text substringWithRange:range]];
}
- (NSRect)firstRectForCharacterRange:(NSRange)range actualRange:(NSRangePointer)actual {
    auto live = state.lock(); NSRect result = NSZeroRect;
    guarded(live, [&] {
        auto input = native_input(live);
        if (!input || !live->text_input->rectangle) return;
        if (range.location == NSNotFound) range = [self selectedRange];
        NSString* text = ns(input->text);
        range.location = MIN(range.location, text.length); range.length = MIN(range.length, text.length - range.location);
        if (actual) *actual = range;
        auto r = live->text_input->rectangle({range.location, range.length});
        result = [live->window convertRectToScreen:[self convertRect:NSMakeRect(r.origin.x, r.origin.y, r.size.width, r.size.height) toView:nil]];
    });
    return result;
}
- (NSUInteger)characterIndexForPoint:(NSPoint)point {
    auto live = state.lock(); NSUInteger result = NSNotFound;
    guarded(live, [&] {
        if (native_input(live) && live->text_input->index) {
            auto p = [self convertPoint:[live->window convertPointFromScreen:point] fromView:nil];
            result = live->text_input->index({p.x, p.y});
        }
    });
    return result;
}
- (void)doCommandBySelector:(SEL)selector { native_text_command(state.lock(), NSStringFromSelector(selector).UTF8String); }
- (void)copy:(id)sender { native_text_command(state.lock(), "copy:"); }
- (void)cut:(id)sender { native_text_command(state.lock(), "cut:"); }
- (void)paste:(id)sender { native_text_command(state.lock(), "paste:"); }
- (void)selectAll:(id)sender { native_text_command(state.lock(), "selectAll:"); }
- (void)undo:(id)sender { native_text_command(state.lock(), "undo:"); }
- (void)redo:(id)sender { native_text_command(state.lock(), "redo:"); }
- (void)delete:(id)sender { native_text_command(state.lock(), "deleteBackward:"); }
- (BOOL)validateUserInterfaceItem:(id<NSValidatedUserInterfaceItem>)item {
    auto input = native_input(state.lock()); if (!input) return NO;
    auto action = item.action;
    if (action == @selector(copy:)) return !input->secure && input->selection.length > 0;
    if (action == @selector(cut:)) return !input->secure && input->editable && input->selection.length > 0;
    if (action == @selector(paste:)) return input->editable && [NSPasteboard.generalPasteboard availableTypeFromArray:@[NSPasteboardTypeString]] != nil;
    if (action == @selector(undo:)) return input->editable && input->can_undo;
    if (action == @selector(redo:)) return input->editable && input->can_redo;
    if (action == @selector(delete:)) return input->editable;
    return YES;
}
- (NSMenu*)menuForEvent:(NSEvent*)event { return native_input(state.lock()) ? edit_menu() : nil; }
- (void)tick:(CADisplayLink*)sender {
    auto live = state.lock();
    guarded(live, [&] { render(live); });
}
- (void)setFrameSize:(NSSize)size {
    [super setFrameSize:size];
    auto live = state.lock();
    guarded(live, [&] {
        if (sync_size(live)) render(live, true);
        update_pause(live);
    });
}
- (void)viewDidChangeBackingProperties {
    [super viewDidChangeBackingProperties];
    auto live = state.lock();
    guarded(live, [&] {
        if (sync_size(live)) render(live, true);
        update_pause(live);
    });
}
- (void)updateTrackingAreas {
    [super updateTrackingAreas];
    if (tracking) [self removeTrackingArea:tracking];
    tracking = [[NSTrackingArea alloc] initWithRect:NSZeroRect
        options:NSTrackingMouseEnteredAndExited | NSTrackingMouseMoved | NSTrackingActiveInKeyWindow | NSTrackingInVisibleRect
        owner:self userInfo:nil];
    [self addTrackingArea:tracking];
}
- (void)pointer:(NSEvent*)event kind:(sw::PointerKind)kind {
    auto live = state.lock();
    guarded(live, [&] {
        auto point = [self convertPoint:event.locationInWindow fromView:nil];
        // AppKit only permits buttonNumber/clickCount on mouse down/up/drag
        // events. Reading them on wheel or tracking events raises NSException.
        bool button_event = false;
        switch (event.type) {
        case NSEventTypeLeftMouseDown: case NSEventTypeLeftMouseUp: case NSEventTypeLeftMouseDragged:
        case NSEventTypeRightMouseDown: case NSEventTypeRightMouseUp: case NSEventTypeRightMouseDragged:
        case NSEventTypeOtherMouseDown: case NSEventTypeOtherMouseUp: case NSEventTypeOtherMouseDragged:
            button_event = true;
            break;
        default: break;
        }
        emit(live, sw::PointerEvent{kind, {point.x, point.y}, button_event ? (int)event.buttonNumber : 0,
            kind == sw::PointerKind::scroll ? event.scrollingDeltaX : 0,
            kind == sw::PointerKind::scroll ? event.scrollingDeltaY : 0, modifiers(event.modifierFlags),
            button_event ? (int)event.clickCount : 0});
        if (live->text_input) [self.inputContext invalidateCharacterCoordinates];
    });
}
- (void)mouseMoved:(NSEvent*)event { [self pointer:event kind:sw::PointerKind::move]; }
- (void)mouseDragged:(NSEvent*)event { [self pointer:event kind:sw::PointerKind::move]; }
- (void)rightMouseDragged:(NSEvent*)event { [self pointer:event kind:sw::PointerKind::move]; }
- (void)otherMouseDragged:(NSEvent*)event { [self pointer:event kind:sw::PointerKind::move]; }
- (void)mouseDown:(NSEvent*)event { [self pointer:event kind:sw::PointerKind::down]; }
- (void)mouseUp:(NSEvent*)event { [self pointer:event kind:sw::PointerKind::up]; }
- (void)rightMouseDown:(NSEvent*)event {
    [self pointer:event kind:sw::PointerKind::down];
    NSMenu* menu = [self menuForEvent:event];
    if (menu) [NSMenu popUpContextMenu:menu withEvent:event forView:self];
}
- (void)rightMouseUp:(NSEvent*)event { [self pointer:event kind:sw::PointerKind::up]; }
- (void)otherMouseDown:(NSEvent*)event { [self pointer:event kind:sw::PointerKind::down]; }
- (void)otherMouseUp:(NSEvent*)event { [self pointer:event kind:sw::PointerKind::up]; }
- (void)scrollWheel:(NSEvent*)event { [self pointer:event kind:sw::PointerKind::scroll]; }
- (void)mouseEntered:(NSEvent*)event { [self pointer:event kind:sw::PointerKind::enter]; }
- (void)mouseExited:(NSEvent*)event { [self pointer:event kind:sw::PointerKind::leave]; }
- (void)keyDown:(NSEvent*)event {
    auto live = state.lock();
    guarded(live, [&] {
        sync_secure_input(live);
        if (live->failed) return;
        const auto input = live->text_input;
        if (input && input->intercept_key &&
            input->intercept_key(sw::KeyEvent{key(event), true, bool(event.isARepeat),
                                                         modifiers(event.modifierFlags), event.keyCode})) {
            sync_secure_input(live);
            return;
        }
        if (native_input(live) && !(key(event) == sw::Key::enter && (event.modifierFlags & NSEventModifierFlagCommand))) {
            [self interpretKeyEvents:@[event]];
        } else emit(live, sw::KeyEvent{key(event), true, bool(event.isARepeat), modifiers(event.modifierFlags), event.keyCode,
            (event.modifierFlags & (NSEventModifierFlagCommand | NSEventModifierFlagControl)) || !event.characters.length || [event.characters characterAtIndex:0] < 32 ? std::string{} : utf8(event.characters)});
    });
}
- (void)keyUp:(NSEvent*)event {
    auto live = state.lock();
    guarded(live, [&] { emit(live, sw::KeyEvent{key(event), false, false, modifiers(event.modifierFlags), event.keyCode}); });
}
- (void)flagsChanged:(NSEvent*)event {
    auto live = state.lock();
    guarded(live, [&] { emit(live, sw::ModifierEvent{modifiers(event.modifierFlags)}); });
}
@end

@implementation SWWindowDelegate
- (BOOL)windowShouldClose:(NSWindow*)window {
    auto live = state.lock();
    guarded(live, [&] {
        if (!live->open || !interactive(live)) return;
        auto request = live->callbacks.on_close_request;
        if (!request || request()) close_surface(live);
    });
    return NO;
}
- (void)windowDidBecomeKey:(NSNotification*)notification {
    auto live = state.lock();
    guarded(live, [&] { activate_menu(live); emit(live, sw::FocusEvent{true}); });
}
- (void)windowDidResignKey:(NSNotification*)notification {
    auto live = state.lock();
    guarded(live, [&] { emit(live, sw::FocusEvent{false}); });
}
- (void)windowDidResize:(NSNotification*)notification {
    auto live = state.lock();
    guarded(live, [&] {
        if (sync_size(live)) render(live, true);
        update_pause(live);
    });
}
- (void)windowDidChangeScreen:(NSNotification*)notification {
    auto live = state.lock();
    guarded(live, [&] { sync_size(live); update_pause(live); });
}
- (void)windowDidChangeOcclusionState:(NSNotification*)notification {
    auto live = state.lock();
    guarded(live, [&] {
        // A newly visible view needs a redraw even when it is not animating.
        live->dirty = true;
        update_pause(live);
    });
}
- (void)windowDidMiniaturize:(NSNotification*)notification {
    auto live = state.lock();
    guarded(live, [&] { update_pause(live); });
}
- (void)windowDidDeminiaturize:(NSNotification*)notification {
    auto live = state.lock();
    guarded(live, [&] { live->dirty = true; update_pause(live); });
}
@end

@implementation SWMenuTarget
- (void)themeToggle:(NSButton*)sender {
    auto live = state.lock(); guarded(live, [&] {
        auto changed = live->theme_changed;
        if (live->open && changed) changed(sender.state == NSControlStateValueOn);
    });
}

- (void)command:(NSMenuItem*)sender {
    auto live = state.lock();
    guarded(live, [&] { invoke(live, (sw::Command)sender.tag); });
}
- (void)newWindow:(id)sender {
    auto app = application.lock();
    guarded(app, [&] { auto callback = app->options.on_new_window; if (callback) callback(); });
}
@end

@implementation SWApplicationDelegate
- (void)applicationDidResignActive:(NSNotification*)notification {
    auto app = state.lock();
    if (!app) return;
    const auto surfaces = app->surfaces;
    for (const auto& weak : surfaces)
        if (auto live = weak.lock(); live && live->open) {
            guarded(live, [&] { emit(live, sw::FocusEvent{false}); });
            sync_secure_input(live, true);
        }
}
- (void)applicationDidBecomeActive:(NSNotification*)notification {
    auto app = state.lock();
    if (!app) return;
    const auto surfaces = app->surfaces;
    for (const auto& weak : surfaces)
        if (auto live = weak.lock(); live && live->open && live->window.keyWindow) {
            guarded(live, [&] { emit(live, sw::FocusEvent{true}); });
            sync_secure_input(live);
        }
}
- (BOOL)applicationShouldTerminateAfterLastWindowClosed:(NSApplication*)sender { return NO; }
- (NSApplicationTerminateReply)applicationShouldTerminate:(NSApplication*)sender {
    auto app = state.lock();
    guarded(app, [&] {
        // Ask before closing anything, so a veto leaves every window intact.
        auto surfaces = app->surfaces;
        for (const auto& weak : surfaces) if (auto live = weak.lock(); live && live->open) {
            auto request = live->callbacks.on_close_request;
            if (request && !request()) return;
        }
        for (const auto& weak : surfaces) close_surface(weak.lock());
        [NSApp stop:nil];
        NSEvent* wake = [NSEvent otherEventWithType:NSEventTypeApplicationDefined location:NSZeroPoint
            modifierFlags:0 timestamp:0 windowNumber:0 context:nil subtype:0 data1:0 data2:0];
        [NSApp postEvent:wake atStart:NO];
    });
    return NSTerminateCancel; // Return through Application::run, allowing C++ destructors to run.
}
- (BOOL)applicationShouldHandleReopen:(NSApplication*)sender hasVisibleWindows:(BOOL)visible {
    auto app = state.lock();
    guarded(app, [&] { if (!visible) { auto callback = app->options.on_new_window; if (callback) callback(); } });
    return YES;
}
@end

namespace skia_window {
Application::Application(std::shared_ptr<detail::ApplicationState> state) : state_(std::move(state)) {}
std::expected<std::unique_ptr<Application>, Error> Application::create(ApplicationOptions options) {
    if (![NSThread isMainThread]) return std::unexpected(Error{"Application must be created on the main thread"});
    @autoreleasepool {
        if (NSApp.delegate) return std::unexpected(Error{"An application delegate is already installed"});
        if (@available(macOS 26.0, *)) {} else return std::unexpected(Error{"macOS 26 or later is required"});
        [NSApplication sharedApplication];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
        auto state = std::make_shared<ApplicationState>();
        state->options = std::move(options);
        state->device = MTLCreateSystemDefaultDevice();
        if (!state->device) return std::unexpected(Error{"No Metal GPU device is available"});
        // Capture must begin before Ganesh creates its first Metal command buffer.
        if (const char* path = std::getenv("SKIA_WINDOW_METAL_CAPTURE")) {
            MTLCaptureManager* manager = MTLCaptureManager.sharedCaptureManager;
            MTLCaptureDescriptor* descriptor = [MTLCaptureDescriptor new];
            descriptor.captureObject = state->device;
            descriptor.destination = MTLCaptureDestinationGPUTraceDocument;
            descriptor.outputURL = [NSURL fileURLWithPath:ns(path)];
            NSError* error = nil;
            state->capture_active = [manager startCaptureWithDescriptor:descriptor error:&error];
            if (!state->capture_active) {
                const char* description = error.localizedDescription.UTF8String;
                report(state, std::string("Metal capture failed: ") + (description ? description : "unknown error"));
            }
        }
        state->queue = [state->device newCommandQueue];
        if (!state->queue) return std::unexpected(Error{"Cannot create the Metal command queue"});
        state->queue.label = @"skia_window GPU queue";
        GrMtlBackendContext metal;
        metal.fDevice.retain((__bridge GrMTLHandle)state->device);
        metal.fQueue.retain((__bridge GrMTLHandle)state->queue);
        state->context = GrDirectContexts::MakeMetal(metal);
        if (!state->context) return std::unexpected(Error{"Cannot create Skia Ganesh's Metal context"});
        state->delegate = [SWApplicationDelegate new];
        state->delegate->state = state;
        NSApp.delegate = state->delegate;
        std::weak_ptr<ApplicationState> weak_state = state;
        state->motion_observer = [NSWorkspace.sharedWorkspace.notificationCenter
            addObserverForName:NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification object:nil
            queue:NSOperationQueue.mainQueue usingBlock:^(NSNotification*) {
                auto app = weak_state.lock();
                guarded(app, [&] {
                    for (const auto& weak : app->surfaces) if (auto surface = weak.lock(); surface && surface->ui) {
                        surface->ui->set_system_reduced_motion(NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion);
                    }
                });
            }];
        state->target = [SWMenuTarget new];
        state->target->application = state;
        state->default_menu = base_menu(state);
        NSApp.mainMenu = state->default_menu;
        [NSApp finishLaunching];
        return std::unique_ptr<Application>(new Application(std::move(state)));
    }
}
Application::~Application() {
    assert([NSThread isMainThread]);
    auto surfaces = state_->surfaces;
    for (const auto& weak : surfaces) close_surface(weak.lock());
    state_->alive = false;
    if (state_->capture_active) {
        [[MTLCaptureManager sharedCaptureManager] stopCapture];
        state_->capture_active = false;
    }
    state_->context->flushAndSubmit(GrSyncCpu::kYes);
    state_->context.reset();
    NSApp.delegate = nil;
    NSApp.mainMenu = nil;
}
int Application::run() {
    main_thread();
    if (state_->running) throw std::logic_error("Application::run cannot be nested");
    state_->running = true;
    [NSApp activate];
    [NSApp run];
    state_->running = false;
    return state_->exit_code;
}
void Application::quit() { main_thread(); [NSApp terminate:nil]; }
void Application::post(std::function<void()> callback) {
    main_thread();
    std::weak_ptr<ApplicationState> weak = state_;
    dispatch_async(dispatch_get_main_queue(), ^{
        auto app = weak.lock();
        guarded(app, [&] { callback(); });
    });
}
std::string Application::gpu_description() const {
    main_thread();
    return std::format("Skia Ganesh / Metal / {}", state_->device.name.UTF8String);
}
Surface::Surface(std::shared_ptr<SurfaceState> state) : state_(std::move(state)) {}
Surface::~Surface() { assert([NSThread isMainThread]); close_surface(state_); }
void Surface::set_cursor(Cursor cursor) {
    main_thread();
    if (!state_->open || state_->view->pointer_cursor == cursor) return;
    state_->view->pointer_cursor = cursor;
    [state_->window invalidateCursorRectsForView:state_->view];
}
void Surface::set_callbacks(Callbacks callbacks) {
    main_thread();
    if (!state_->open) return;
    state_->callbacks = std::move(callbacks);
    state_->failed = false;
    request_redraw();
}
void Surface::set_accessibility_provider(std::shared_ptr<AccessibilityProvider> provider) {
    main_thread();if(!state_->open)return;
    state_->accessibility=std::move(provider);state_->accessibility_generation=0;state_->view->accessibility_nodes=nil;
    NSAccessibilityPostNotification(state_->view,NSAccessibilityLayoutChangedNotification);
}
void Surface::set_text_input_client(std::shared_ptr<TextInputClient> client) {
    main_thread(); if (!state_->open) return;
    const auto previous = state_->text_input;
    if (previous && previous->unmark) previous->unmark(true);
    if (previous && previous->deactivate) previous->deactivate();
    state_->text_input = std::move(client); install_edit_menu(state_);
    sync_secure_input(state_);
    [state_->view.inputContext invalidateCharacterCoordinates];
}
std::string clipboard_text() { main_thread(); return utf8([NSPasteboard.generalPasteboard stringForType:NSPasteboardTypeString]); }
void set_clipboard_text(const std::string& text) {
    main_thread(); [NSPasteboard.generalPasteboard clearContents];
    [NSPasteboard.generalPasteboard setString:ns(text) forType:NSPasteboardTypeString];
}
void Surface::set_view_builder(std::function<ui::View()> builder) {
    main_thread();
    auto state = state_;
    if (!state->open) throw std::logic_error("Cannot mount content on a closed surface");
    if (state->text_input && state->text_input->unmark) state->text_input->unmark(true);
    state->text_input.reset();
    const auto epoch = state->ui_mount_epoch + 1;
    std::weak_ptr<SurfaceState> weak = state;
    auto host = std::make_shared<ui::Host>(std::move(builder), [weak, epoch] {
        if (auto live = weak.lock(); live && live->open && live->ui_mount_epoch == epoch) { live->dirty = true; update_pause(live); }
    });
    host->set_system_reduced_motion(NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion);
    host->layout(detail::content_size(state));
    if (!state->open) throw std::logic_error("Surface closed while mounting content");
    state->view->accessibility_nodes = nil;
    state->ui_mount_epoch = epoch;
    state->ui_generation = 0;
    state->ui = std::move(host);
    state->accessibility.reset();
    state->dirty = true;
    update_pause(state);
}
void Surface::set_content(ui::View content) { set_view_builder([content = std::move(content)] { return content; }); }
void Surface::clear_content() {
    main_thread();
    auto state = state_;
    ++state->ui_mount_epoch;
    state->ui.reset();
    if (state->view) {
        state->view->accessibility_nodes = nil;
        NSAccessibilityPostNotification(state->view, NSAccessibilityLayoutChangedNotification);
    }
    request_redraw();
}
std::shared_ptr<ui::Host> Surface::ui_host() const { main_thread(); return state_->ui; }
void Surface::set_title(std::string title) { main_thread(); state_->title = std::move(title); state_->window.title = ns(state_->title); }
void Surface::show() {
    main_thread();
    auto state = state_; // Focus callbacks may destroy the public wrapper synchronously.
    if (!state->open || state->modal) return;
    if (state->window.miniaturized) [state->window deminiaturize:nil];
    [state->window makeKeyAndOrderFront:nil];
    [state->window makeFirstResponder:state->view];
    state->dirty = true;
    update_pause(state);
}
void Surface::hide() {
    main_thread();
    auto state = state_;
    if (state->modal) throw std::logic_error("Dismiss a modal dialog instead of hiding it");
    [state->window orderOut:nil]; update_pause(state);
    const auto input = state->text_input;
    if (input && input->deactivate) input->deactivate();
    sync_secure_input(state, true);
}
void Surface::minimize() {
    main_thread();
    const auto state = state_;
    if (state->open && (state->window.styleMask & NSWindowStyleMaskMiniaturizable)) {
        [state->window miniaturize:nil];
        const auto input = state->text_input;
        if (input && input->deactivate) input->deactivate();
        sync_secure_input(state, true);
    }
}
bool Surface::is_minimized() const { main_thread(); return state_->open && state_->window.miniaturized; }
void Surface::focus() {
    main_thread();
    auto state = state_;
    if (state->open) { [state->window makeKeyAndOrderFront:nil]; if (state->open) activate_menu(state); }
}
void Surface::close() { main_thread(); close_surface(state_); }
bool Surface::is_open() const { main_thread(); return state_->open; }
void Surface::request_redraw() { main_thread(); if (state_->open) { state_->dirty = true; update_pause(state_); } }
void Surface::set_redraw_delay(std::optional<std::chrono::milliseconds> delay) {
    main_thread();
    const auto generation = ++state_->redraw_timer_generation;
    if (!state_->open || !delay) return;
    if (delay->count() < 0 || *delay > std::chrono::hours(24))
        throw std::invalid_argument("Redraw delay must be between zero and one day");
    const auto weak = std::weak_ptr<SurfaceState>(state_);
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, delay->count() * NSEC_PER_MSEC), dispatch_get_main_queue(), ^{
        if (auto live = weak.lock(); live && live->open && live->redraw_timer_generation == generation) {
            live->dirty = true;
            update_pause(live);
        }
    });
}
void Surface::set_animating(bool value) { main_thread(); state_->animating = value && state_->open; state_->previous = {}; request_redraw(); }
bool Surface::is_animating() const { main_thread(); return state_->animating; }
Size Surface::content_size() const { main_thread(); return detail::content_size(state_); }
std::expected<std::unique_ptr<Window>, Error> Window::create(Application& app, WindowOptions options) {
    auto state = create_surface(app.state_, std::move(options));
    if (!state) return std::unexpected(state.error());
    return std::unique_ptr<Window>(new Window(std::move(*state)));
}
Window::~Window() { close_surface(state_); }
std::expected<void, Error> Window::set_menu(std::span<const MenuItem> items, std::function<void(Command)> callback) {
    main_thread();
    if (!state_->open) return std::unexpected(Error{"Window is closed"});
    if (auto valid = validate_menu(items); !valid) return valid;
    auto app = state_->application.lock();
    if (!app) return std::unexpected(Error{"Application is closed"});
    state_->menu_items.assign(items.begin(), items.end());
    state_->on_command = std::move(callback);
    NSMenu* bar = base_menu(app);
    NSMenu* entries = build_menu(state_->menu_items, state_->target);
    for (NSMenuItem* item in [entries.itemArray copy]) { [entries removeItem:item]; [bar addItem:item]; }
    state_->menu = bar;
    install_edit_menu(state_);
    if (state_->window.keyWindow || NSApp.keyWindow.sheetParent == state_->window) activate_menu(state_);
    return {};
}
void Window::set_command_state(Command command, bool enabled, bool checked) {
    main_thread();
    auto* item = find_command(state_->menu_items, command);
    if (!item) return;
    item->enabled = enabled;
    item->checked = checked;
    NSMenuItem* native = native_command(state_->menu, command);
    native.enabled = enabled;
    native.state = checked ? NSControlStateValueOn : NSControlStateValueOff;
}
bool Window::invoke_command(Command command) { main_thread(); return invoke(state_, command); }
void Window::set_appearance(Appearance appearance) {
    main_thread(); state_->window.appearance = appearance == Appearance::system ? nil :
        [NSAppearance appearanceNamed:appearance == Appearance::light ? NSAppearanceNameAqua : NSAppearanceNameDarkAqua];
}
void Window::set_titlebar_theme_toggle(bool light, std::function<void(bool)> changed) {
    main_thread(); if (!state_->open) return;
    state_->theme_changed = std::move(changed);
    if (!state_->theme_accessory) {
        auto accessory = [NSTitlebarAccessoryViewController new];
        accessory.layoutAttribute = NSLayoutAttributeRight;
        // Match the native titlebar height; a shorter accessory is bottom-aligned by AppKit.
        NSView* holder = [[NSView alloc] initWithFrame:NSMakeRect(0, 0, 42, 32)];
        NSButton* button = [[NSButton alloc] initWithFrame:NSMakeRect(6, 5, 30, 22)];
        button.autoresizingMask = NSViewMinXMargin | NSViewMaxXMargin | NSViewMinYMargin | NSViewMaxYMargin;
        button.buttonType = NSButtonTypePushOnPushOff; button.bezelStyle = NSBezelStyleAccessoryBarAction;
        button.imagePosition = NSImageOnly; button.target = state_->target; button.action = @selector(themeToggle:);
        button.accessibilityLabel = @"Light mode"; button.accessibilityIdentifier = @"scars-titlebar-theme-toggle";
        [holder addSubview:button]; accessory.view = holder;
        [state_->window addTitlebarAccessoryViewController:accessory];
        state_->theme_accessory = accessory; state_->theme_button = button;
    }
    set_titlebar_theme_toggle_state(light);
}
void Window::set_titlebar_theme_toggle_state(bool light) {
    main_thread(); auto button = state_->theme_button; if (!button) return;
    button.state = light ? NSControlStateValueOn : NSControlStateValueOff;
    button.image = [NSImage imageWithSystemSymbolName:light ? @"sun.max.fill" : @"moon.fill" accessibilityDescription:nil];
    button.toolTip = light ? @"Switch to dark mode" : @"Switch to light mode";
}
std::expected<std::unique_ptr<Dialog>, Error> Dialog::create(Window& parent, WindowOptions options) {
    auto state = create_surface(parent.state_->application.lock(), std::move(options), parent.state_);
    if (!state) return std::unexpected(state.error());
    return std::unique_ptr<Dialog>(new Dialog(std::move(*state)));
}
Dialog::~Dialog() { close_surface(state_); }
std::expected<void, Error> Dialog::show_modal(std::function<void(DialogResult)> completion) {
    main_thread();
    auto state = state_;
    auto parent = state->parent.lock();
    if (!state->open || !parent || !parent->open) return std::unexpected(Error{"Dialog or parent is closed"});
    if (state->modal || parent->window.attachedSheet) return std::unexpected(Error{"A modal dialog is already presented for this parent"});
    state->modal = true;
    state->completion = std::move(completion);
    [state->window orderOut:nil];
    if (!state->open || !parent->open) return std::unexpected(Error{"Dialog closed during presentation"});
    [parent->window beginSheet:state->window completionHandler:nil];
    if (state->open) {
        [state->window makeFirstResponder:state->view];
        activate_menu(state);
        state->dirty = true;
        update_pause(state);
    }
    return {};
}
void Dialog::dismiss(DialogResult result) { main_thread(); close_surface(state_, result); }
} // namespace skia_window
