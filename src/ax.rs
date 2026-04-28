#![cfg(target_os = "macos")]

use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::{CFString, CFStringRef};

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
    fn AXUIElementCopyAttributeValue(
        element: CFTypeRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
}

const AX_OK: i32 = 0;

pub fn prompt_trust() -> bool {
    unsafe {
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let val = CFBoolean::true_value();
        let dict = CFDictionary::from_CFType_pairs(&[(key, val)]);
        AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef())
    }
}

pub fn is_trusted() -> bool {
    // Pass NULL options. Empty CFDictionary segfaults inside CFGetTypeID
    // on macOS 26 when called from LSUIElement bundles.
    unsafe { AXIsProcessTrustedWithOptions(std::ptr::null()) }
}

pub fn window_title_for_pid(pid: i32) -> Option<String> {
    unsafe {
        let app = AXUIElementCreateApplication(pid);
        if app.is_null() {
            return None;
        }
        let focused_attr = CFString::new("AXFocusedWindow");
        let title_attr = CFString::new("AXTitle");

        let mut window: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(app, focused_attr.as_concrete_TypeRef(), &mut window);
        if err != AX_OK || window.is_null() {
            if err == -25204 {
                eprintln!("ax: kAXErrorAPIDisabled (no Accessibility permission)");
            } else if err != AX_OK {
                eprintln!("ax: AXUIElementCopyAttributeValue(focused) err={err}");
            }
            CFRelease(app);
            return None;
        }
        let mut title: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(window, title_attr.as_concrete_TypeRef(), &mut title);
        let result = if err == AX_OK && !title.is_null() {
            let s = CFString::wrap_under_create_rule(title as CFStringRef);
            Some(s.to_string())
        } else {
            None
        };
        CFRelease(window);
        CFRelease(app);
        result
    }
}
