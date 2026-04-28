#![cfg(target_os = "macos")]

use crossbeam_channel::Sender;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObject, NSObjectProtocol, NSString};

pub struct ReopenIvars {
    pub tx: Sender<()>,
}

declare_class!(
    pub struct ReopenObserver;

    unsafe impl ClassType for ReopenObserver {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "FocusTraceReopenObserver";
    }

    impl DeclaredClass for ReopenObserver {
        type Ivars = ReopenIvars;
    }

    unsafe impl NSObjectProtocol for ReopenObserver {}

    unsafe impl ReopenObserver {
        #[method(activated:)]
        fn activated(&self, _notification: &NSNotification) {
            let _ = self.ivars().tx.try_send(());
        }
    }
);

impl ReopenObserver {
    pub fn new(tx: Sender<()>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(ReopenIvars { tx });
        unsafe { msg_send_id![super(this), init] }
    }
}

pub fn install(tx: Sender<()>) -> Retained<ReopenObserver> {
    unsafe {
        let observer = ReopenObserver::new(tx);
        let nc = NSNotificationCenter::defaultCenter();
        let name = NSString::from_str("NSApplicationDidBecomeActiveNotification");
        let sel = objc2::sel!(activated:);
        let proto: &ProtocolObject<dyn NSObjectProtocol> = ProtocolObject::from_ref(&*observer);
        let _: () = objc2::msg_send![&nc, addObserver: proto as *const _, selector: sel, name: &*name, object: std::ptr::null::<AnyObject>()];
        observer
    }
}
