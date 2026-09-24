use android_activity::AndroidApp;
use jni::objects::{Global, JObject, JValue};
use jni::{JavaVM, jni_sig, jni_str};
use log::error;

use crate::Rect;

/// Returns the drawable bounds of the window, shrunk to avoid system UI
/// (status bar, navigation bar, display cutouts).
///
/// Falls back to the full window dimensions on JNI errors.
pub fn get_bounds(app: &AndroidApp) -> Option<Rect> {
    let window = app.native_window()?;
    let win_width = window.width() as f32;
    let win_height = window.height() as f32;

    let vm_ptr = app.vm_as_ptr();
    let activity_ptr = app.activity_as_ptr();
    if vm_ptr.is_null() || activity_ptr.is_null() {
        return Some(Rect::from_xywh(0.0, 0.0, win_width, win_height));
    }

    let vm = unsafe { JavaVM::from_raw(vm_ptr.cast()) };
    let raw_activity = activity_ptr as jni::sys::jobject;

    let result = vm.attach_current_thread(|env| -> jni::errors::Result<Rect> {
        let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&raw_activity)? };

        // --- SDK version ---
        let sdk_int = env
            .get_static_field(
                jni_str!("android/os/Build$VERSION"),
                jni_str!("SDK_INT"),
                jni_sig!("I"),
            )
            .and_then(|v| v.i())
            .unwrap_or(0);

        // --- Obtain a WindowInsets object ---
        //
        // Primary source: decor-view's root insets (available once the view is attached).
        // Fallback (API 30+): WindowMetrics, which works even before the view is laid out.
        let window_obj = env
            .call_method(
                &activity,
                jni_str!("getWindow"),
                jni_sig!("()Landroid/view/Window;"),
                &[],
            )?
            .l()?;

        let decor_view = if !window_obj.as_raw().is_null() {
            env.call_method(
                &window_obj,
                jni_str!("getDecorView"),
                jni_sig!("()Landroid/view/View;"),
                &[],
            )?
            .l()?
        } else {
            JObject::null()
        };

        let mut insets_obj = JObject::null();
        if !decor_view.as_raw().is_null() {
            insets_obj = env
                .call_method(
                    &decor_view,
                    jni_str!("getRootWindowInsets"),
                    jni_sig!("()Landroid/view/WindowInsets;"),
                    &[],
                )?
                .l()?;
        }

        if insets_obj.as_raw().is_null() && sdk_int >= 30 {
            let wm = env
                .call_method(
                    &activity,
                    jni_str!("getWindowManager"),
                    jni_sig!("()Landroid/view/WindowManager;"),
                    &[],
                )?
                .l()?;
            if !wm.as_raw().is_null() {
                let metrics = env
                    .call_method(
                        &wm,
                        jni_str!("getCurrentWindowMetrics"),
                        jni_sig!("()Landroid/view/WindowMetrics;"),
                        &[],
                    )?
                    .l()?;
                if !metrics.as_raw().is_null() {
                    insets_obj = env
                        .call_method(
                            &metrics,
                            jni_str!("getWindowInsets"),
                            jni_sig!("()Landroid/view/WindowInsets;"),
                            &[],
                        )?
                        .l()?;
                }
            }
        }

        // --- Extract LTRB inset values ---
        let (left, top, right, bottom) = if insets_obj.as_raw().is_null() {
            (0.0_f32, 0.0, 0.0, 0.0)
        } else if sdk_int >= 30 {
            // WindowInsets.Type flags (API 30+):
            //   statusBars()     = 1
            //   navigationBars() = 2
            //   displayCutout()  = 128
            // Combining all three gives the tightest safe area.
            const TYPE_ALL: i32 = 1 | 2 | 128;

            let insets = env
                .call_method(
                    &insets_obj,
                    jni_str!("getInsets"),
                    jni_sig!("(I)Landroid/graphics/Insets;"),
                    &[JValue::Int(TYPE_ALL)],
                )?
                .l()?;

            if insets.as_raw().is_null() {
                (0.0, 0.0, 0.0, 0.0)
            } else {
                let l = env
                    .get_field(&insets, jni_str!("left"), jni_sig!("I"))?
                    .i()? as f32;
                let t = env
                    .get_field(&insets, jni_str!("top"), jni_sig!("I"))?
                    .i()? as f32;
                let r = env
                    .get_field(&insets, jni_str!("right"), jni_sig!("I"))?
                    .i()? as f32;
                let b = env
                    .get_field(&insets, jni_str!("bottom"), jni_sig!("I"))?
                    .i()? as f32;
                (l, t, r, b)
            }
        } else {
            // Pre-API 30: getSystemWindowInset*() covers status + nav bars.
            // Display cutouts were introduced in API 28 but are typically captured
            // inside the system window insets on those versions.
            let l = env
                .call_method(
                    &insets_obj,
                    jni_str!("getSystemWindowInsetLeft"),
                    jni_sig!("()I"),
                    &[],
                )?
                .i()? as f32;
            let t = env
                .call_method(
                    &insets_obj,
                    jni_str!("getSystemWindowInsetTop"),
                    jni_sig!("()I"),
                    &[],
                )?
                .i()? as f32;
            let r = env
                .call_method(
                    &insets_obj,
                    jni_str!("getSystemWindowInsetRight"),
                    jni_sig!("()I"),
                    &[],
                )?
                .i()? as f32;
            let b = env
                .call_method(
                    &insets_obj,
                    jni_str!("getSystemWindowInsetBottom"),
                    jni_sig!("()I"),
                    &[],
                )?
                .i()? as f32;
            (l, t, r, b)
        };

        Ok(Rect::from_ltrb(
            left,
            top,
            win_width - right,
            win_height - bottom,
        ))
    });

    match result {
        Ok(rect) => Some(rect),
        Err(e) => {
            error!("Failed to retrieve bounds via JNI: {e:?}");
            Some(Rect::from_xywh(0.0, 0.0, win_width, win_height))
        }
    }
}
