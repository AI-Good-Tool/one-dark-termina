//! macOS Dock 右键菜单：注入"新建窗口"项。
//! Tauri 2 尚无官方 Dock 菜单 API，这里用 Objective-C 运行时给 App 委托类
//! 动态添加 `applicationDockMenu:` 方法，返回一个含"新建窗口"的 NSMenu。
#![cfg(target_os = "macos")]

use cocoa::appkit::{NSMenu, NSMenuItem};
use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use objc::declare::ClassDecl;
use objc::runtime::{class_addMethod, object_getClass, Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use tauri::AppHandle;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
// NSMenu / 目标对象指针（intentional leak：与 App 同生命周期）
static DOCK_MENU: AtomicUsize = AtomicUsize::new(0);
static HANDLER: AtomicUsize = AtomicUsize::new(0);

/// 菜单项"新建窗口"被点击
extern "C" fn on_new_window(_this: &Object, _cmd: Sel, _sender: id) {
    if let Some(app) = APP_HANDLE.get() {
        if let Err(e) = crate::create_window(app, "index.html") {
            eprintln!("dock 新建窗口失败: {e}");
        }
    }
}

/// App 委托方法：返回 Dock 右键菜单
extern "C" fn application_dock_menu(_this: &Object, _cmd: Sel, _sender: id) -> id {
    DOCK_MENU.load(Ordering::SeqCst) as id
}

/// 安装 Dock 菜单。title 为本地化后的"新建窗口"文案。
pub fn install(app: &AppHandle, title: &str) {
    let _ = APP_HANDLE.set(app.clone());
    unsafe {
        // 1. 目标对象类：响应菜单点击
        let mut decl = match ClassDecl::new("ODTDockMenuTarget", class!(NSObject)) {
            Some(d) => d,
            None => return,
        };
        decl.add_method(
            sel!(odtNewWindow:),
            on_new_window as extern "C" fn(&Object, Sel, id),
        );
        let target_cls = decl.register();
        let target: id = msg_send![target_cls, new];
        HANDLER.store(target as usize, Ordering::SeqCst);

        // 2. 构建菜单：新建窗口
        let menu = NSMenu::new(nil);
        let ns_title = NSString::alloc(nil).init_str(title);
        let item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            ns_title,
            sel!(odtNewWindow:),
            NSString::alloc(nil).init_str(""),
        );
        let () = msg_send![item, setTarget: target];
        menu.addItem_(item);
        DOCK_MENU.store(menu as usize, Ordering::SeqCst);

        // 3. 给 App 委托类动态添加 applicationDockMenu: 方法
        let nsapp: id = msg_send![class!(NSApplication), sharedApplication];
        let delegate: id = msg_send![nsapp, delegate];
        if delegate == nil {
            return;
        }
        let cls: *mut Class = object_getClass(delegate) as *mut Class;
        let imp = application_dock_menu as extern "C" fn(&Object, Sel, id) -> id;
        let ok = class_addMethod(
            cls,
            sel!(applicationDockMenu:),
            std::mem::transmute::<extern "C" fn(&Object, Sel, id) -> id, objc::runtime::Imp>(imp),
            b"@@:@\0".as_ptr() as *const i8,
        );
        if ok == objc::runtime::NO {
            eprintln!("dock 菜单注入失败：applicationDockMenu: 已存在");
        }
    }
}
