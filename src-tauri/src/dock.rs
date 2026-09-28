//! macOS Dock 右键菜单：打开的终端窗口列表（点击切换/聚焦）+ "新建窗口"。
//! Tauri 2 尚无官方 Dock 菜单 API，这里用 Objective-C 运行时给 App 委托类
//! 动态添加 `applicationDockMenu:` 方法。该方法在每次右键 Dock 图标时被系统
//! 调用，因此菜单每次现建，窗口列表始终是当前的——合并到只剩一个窗口时
//! 自然只显示一项。
#![cfg(target_os = "macos")]

use cocoa::appkit::{NSMenu, NSMenuItem};
use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use objc::declare::ClassDecl;
use objc::runtime::{class_addMethod, object_getClass, Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static NEW_WINDOW_TITLE: OnceLock<String> = OnceLock::new();
/// 本次菜单构建的窗口标签快照：菜单项 tag 是下标，点击时反查
static WINDOW_LABELS: Mutex<Vec<String>> = Mutex::new(Vec::new());
// 目标对象指针（intentional leak：与 App 同生命周期）
static HANDLER: AtomicUsize = AtomicUsize::new(0);

/// 菜单项"新建窗口"被点击
extern "C" fn on_new_window(_this: &Object, _cmd: Sel, _sender: id) {
    if let Some(app) = APP_HANDLE.get() {
        if let Err(e) = crate::create_window(app, "index.html") {
            eprintln!("dock 新建窗口失败: {e}");
        }
    }
}

/// 窗口列表项被点击：聚焦对应窗口（最小化则先还原）
extern "C" fn on_focus_window(_this: &Object, _cmd: Sel, sender: id) {
    unsafe {
        let tag: isize = msg_send![sender, tag];
        let label = WINDOW_LABELS
            .lock()
            .ok()
            .and_then(|labels| labels.get(tag as usize).cloned());
        let (Some(label), Some(app)) = (label, APP_HANDLE.get()) else {
            return;
        };
        if let Some(win) = app.get_webview_window(&label) {
            if win.is_minimized().unwrap_or(false) {
                let _ = win.unminimize();
            }
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

/// App 委托方法：每次右键 Dock 图标时被调用，现建菜单（窗口列表 + 新建窗口）
extern "C" fn application_dock_menu(_this: &Object, _cmd: Sel, _sender: id) -> id {
    let Some(app) = APP_HANDLE.get() else {
        return nil;
    };
    unsafe {
        let menu = NSMenu::new(nil);
        let handler = HANDLER.load(Ordering::SeqCst) as id;
        let empty = NSString::alloc(nil).init_str("");

        // 终端窗口（main + win-*；编辑器 editor-* 不列出），main 在前
        let mut wins: Vec<_> = app
            .webview_windows()
            .into_iter()
            .filter(|(l, _)| l == "main" || l.starts_with("win-"))
            .collect();
        wins.sort_by(|a, b| a.0.cmp(&b.0));
        if let Ok(mut labels) = WINDOW_LABELS.lock() {
            *labels = wins.iter().map(|(l, _)| l.clone()).collect();
        }

        for (i, (_label, win)) in wins.iter().enumerate() {
            let mut title = win.title().unwrap_or_default();
            if title.is_empty() {
                title = "One Dark Terminal".to_string();
            }
            let ns_title = NSString::alloc(nil).init_str(&title);
            let item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                ns_title,
                sel!(odtFocusWindow:),
                empty,
            );
            let () = msg_send![item, setTarget: handler];
            let () = msg_send![item, setTag: i as isize];
            // 当前聚焦窗口打勾（macOS 窗口列表惯例）
            if win.is_focused().unwrap_or(false) {
                let () = msg_send![item, setState: 1_isize];
            }
            menu.addItem_(item);
            let _: id = msg_send![item, autorelease];
            let _: id = msg_send![ns_title, autorelease];
        }

        if !wins.is_empty() {
            let sep = NSMenuItem::separatorItem(nil);
            menu.addItem_(sep);
        }

        let title = NEW_WINDOW_TITLE.get().map(|s| s.as_str()).unwrap_or("New Window");
        let ns_title = NSString::alloc(nil).init_str(title);
        let item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            ns_title,
            sel!(odtNewWindow:),
            empty,
        );
        let () = msg_send![item, setTarget: handler];
        menu.addItem_(item);
        let _: id = msg_send![item, autorelease];
        let _: id = msg_send![ns_title, autorelease];
        let _: id = msg_send![empty, autorelease];

        let menu: id = msg_send![menu, autorelease];
        menu
    }
}

/// 安装 Dock 菜单。title 为本地化后的"新建窗口"文案。
pub fn install(app: &AppHandle, title: &str) {
    let _ = APP_HANDLE.set(app.clone());
    let _ = NEW_WINDOW_TITLE.set(title.to_string());
    unsafe {
        // 1. 目标对象类：响应菜单点击（新建窗口 / 聚焦窗口）
        let mut decl = match ClassDecl::new("ODTDockMenuTarget", class!(NSObject)) {
            Some(d) => d,
            None => return,
        };
        decl.add_method(
            sel!(odtNewWindow:),
            on_new_window as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(odtFocusWindow:),
            on_focus_window as extern "C" fn(&Object, Sel, id),
        );
        let target_cls = decl.register();
        let target: id = msg_send![target_cls, new];
        HANDLER.store(target as usize, Ordering::SeqCst);

        // 2. 给 App 委托类动态添加 applicationDockMenu: 方法
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
