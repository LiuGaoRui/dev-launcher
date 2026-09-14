//! WebView2 内存调优（ADR-007）。
//!
//! 窗口失焦时把 WebView2 的内存目标级别降为 Low：WebView2 会把部分浏览器
//! 进程内存换出磁盘，JS 脚本仍继续运行（前端轮询与日志订阅不受影响）；
//! 聚焦时恢复 Normal。需要 WebView2 Runtime >= 114.0.1823.32，更旧的运行时
//! 上接口 cast 失败，静默跳过即可，无副作用。

#[cfg(windows)]
use windows_core::Interface;
#[cfg(windows)]
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
};

/// 按窗口焦点切换 WebView2 内存目标级别：聚焦 Normal / 失焦 Low。
pub fn on_focus_change<R: tauri::Runtime>(window: &tauri::Window<R>, focused: bool) {
    #[cfg(windows)]
    {
        for webview in window.webviews() {
            // 闭包投递到主线程执行；级别切换是幂等轻量调用，无需去抖
            let _ = webview.with_webview(move |platform| {
                // Safety: COM 指针由 wry 创建并随窗口存活，此处仅同步方法调用
                unsafe {
                    let Ok(core) = platform.controller().CoreWebView2() else {
                        return;
                    };
                    let Ok(w19) = core.cast::<ICoreWebView2_19>() else {
                        return; // 运行时 < 114，不支持该接口
                    };
                    let level = if focused {
                        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
                    } else {
                        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
                    };
                    let _ = w19.SetMemoryUsageTargetLevel(level);
                }
            });
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (window, focused);
    }
}
