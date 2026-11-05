// Standalone AppKit diagnostic: deliberately uses neither SCARS nor Skia.
#import <AppKit/AppKit.h>
#import <QuartzCore/QuartzCore.h>
#include <iostream>

@interface ScarsDesktopProbe : NSObject
@property int ticks;
- (void)tick:(CADisplayLink*)link;
@end
@implementation ScarsDesktopProbe
- (void)tick:(CADisplayLink*)link {
    ++self.ticks;
}
@end

int main() {
    @autoreleasepool {
        auto app = [NSApplication sharedApplication];
        const bool policy = [app setActivationPolicy:NSApplicationActivationPolicyRegular];
        auto window =
            [[NSWindow alloc] initWithContentRect:NSMakeRect(0, 0, 320, 240)
                                        styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable
                                          backing:NSBackingStoreBuffered
                                            defer:NO];
        window.releasedWhenClosed = NO;
        window.title = @"SCARS Desktop Probe";
        auto target = [ScarsDesktopProbe new];
        auto link = [window.contentView displayLinkWithTarget:target selector:@selector(tick:)];
        [link addToRunLoop:NSRunLoop.mainRunLoop forMode:NSRunLoopCommonModes];
        [window center];
        [window makeKeyAndOrderFront:nil];
        [app finishLaunching];
        [app activate];
        __block bool ready = false;
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, NSEC_PER_SEC), dispatch_get_main_queue(), ^{
          uint32_t displays{};
          const auto error = CGGetActiveDisplayList(0, nullptr, &displays);
          auto front = NSWorkspace.sharedWorkspace.frontmostApplication;
          ready = policy && error == kCGErrorSuccess && displays > 0 && app.active && window.keyWindow &&
                  target.ticks > 0;
          std::cout << "SCARS desktop: active-displays=" << displays << " display-error=" << error
                    << " front=" << (front.bundleIdentifier.UTF8String ?: "none")
                    << " app-active=" << bool(app.active) << " key-window=" << bool(window.keyWindow)
                    << " visible=" << bool(window.visible) << " link-paused=" << bool(link.paused)
                    << " callbacks=" << target.ticks << " ready=" << ready << std::endl;
          [app stop:nil];
          [app postEvent:[NSEvent otherEventWithType:NSEventTypeApplicationDefined
                                            location:NSZeroPoint
                                       modifierFlags:0
                                           timestamp:0
                                        windowNumber:0
                                             context:nil
                                             subtype:0
                                               data1:0
                                               data2:0]
                 atStart:NO];
        });
        [app run];
        [link invalidate];
        [window close];
        return ready ? 0 : 1;
    }
}
