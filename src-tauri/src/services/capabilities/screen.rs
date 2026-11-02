use mytools_core::capability::{Capability, CapabilityError, CapabilityRegistry, Permission};

pub(super) fn register(registry: &mut CapabilityRegistry) -> Result<(), CapabilityError> {
    registry.register(Capability::new(
        "screen.pickcolor",
        Permission::Exec,
        "Read the pixel at the current cursor",
        Box::new(|_| sample()),
    ))?;
    Ok(())
}

fn sample() -> Result<serde_json::Value, CapabilityError> {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC};
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut point = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut point) == 0 {
            return Err(CapabilityError::handler_failed(
                "screen.pickcolor",
                "无法获取光标位置",
            ));
        }
        let dc = GetDC(std::ptr::null_mut());
        if dc.is_null() {
            return Err(CapabilityError::handler_failed(
                "screen.pickcolor",
                "无法访问屏幕",
            ));
        }
        let color = GetPixel(dc, point.x, point.y);
        ReleaseDC(std::ptr::null_mut(), dc);
        if color == u32::MAX {
            return Err(CapabilityError::handler_failed(
                "screen.pickcolor",
                "无法读取像素",
            ));
        }
        let (r, g, b) = (color & 255, (color >> 8) & 255, (color >> 16) & 255);
        Ok(
            serde_json::json!({"hex":format!("#{r:02X}{g:02X}{b:02X}"),"rgb":[r,g,b],"x":point.x,"y":point.y}),
        )
    }
    #[cfg(not(windows))]
    Err(CapabilityError::handler_failed(
        "screen.pickcolor",
        "仅支持 Windows",
    ))
}
