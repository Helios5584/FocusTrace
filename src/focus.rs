use crate::db::FocusEvent;
use chrono::Utc;
use crossbeam_channel::Sender;

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, ProtocolObject};
    use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
    use objc2_app_kit::{NSRunningApplication, NSWorkspace};
    use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol, NSString};
    use std::cell::RefCell;

    pub struct ObserverIvars {
        pub tx: Sender<FocusEvent>,
        pub last_app: RefCell<String>,
    }

    declare_class!(
        pub struct FocusObserver;

        unsafe impl ClassType for FocusObserver {
            type Super = NSObject;
            type Mutability = mutability::InteriorMutable;
            const NAME: &'static str = "FocusTraceObserver";
        }

        impl DeclaredClass for FocusObserver {
            type Ivars = ObserverIvars;
        }

        unsafe impl NSObjectProtocol for FocusObserver {}

        unsafe impl FocusObserver {
            #[method(activated:)]
            fn activated(&self, notification: &NSNotification) {
                unsafe {
                    let user_info = notification.userInfo();
                    let Some(info) = user_info else { return };
                    let key = NSString::from_str("NSWorkspaceApplicationKey");
                    let value = info.objectForKey(&key);
                    let Some(obj) = value else { return };
                    let app: &NSRunningApplication = &*(Retained::as_ptr(&obj) as *const NSRunningApplication);
                    let name = app.localizedName().map(|s| s.to_string()).unwrap_or_default();
                    let bundle = app.bundleIdentifier().map(|s| s.to_string()).unwrap_or_default();

                    let prev = self.ivars().last_app.replace(name.clone());

                    let pid: i32 = objc2::msg_send![app, processIdentifier];
                    let title = crate::ax::window_title_for_pid(pid).unwrap_or_default();

                    let ev = FocusEvent {
                        id: 0,
                        ts: Utc::now(),
                        app_name: name,
                        bundle_id: bundle,
                        window_title: title,
                        previous_app: prev,
                    };
                    let _ = self.ivars().tx.try_send(ev);
                }
            }
        }
    );

    impl FocusObserver {
        pub fn new(tx: Sender<FocusEvent>) -> Retained<Self> {
            let this = Self::alloc().set_ivars(ObserverIvars {
                tx,
                last_app: RefCell::new(String::new()),
            });
            unsafe { msg_send_id![super(this), init] }
        }
    }

    pub fn install(tx: Sender<FocusEvent>) -> Retained<FocusObserver> {
        unsafe {
            let observer = FocusObserver::new(tx);
            let workspace = NSWorkspace::sharedWorkspace();
            let nc = workspace.notificationCenter();
            let name = NSString::from_str("NSWorkspaceDidActivateApplicationNotification");
            let sel = objc2::sel!(activated:);
            let proto: &ProtocolObject<dyn NSObjectProtocol> = ProtocolObject::from_ref(&*observer);
            let _: () = objc2::msg_send![&nc, addObserver: proto as *const _, selector: sel, name: &*name, object: std::ptr::null::<AnyObject>()];
            observer
        }
    }
}

#[cfg(target_os = "macos")]
pub use imp::install;

#[cfg(not(target_os = "macos"))]
pub fn install(_tx: Sender<FocusEvent>) {}
