use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub offset: u32,
    pub limit: Option<u32>,
    pub sort: Option<u32>,
    pub instance: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub path: String,
    pub name: String,
    pub directory: bool,
    pub size: u64,
    pub modified: u64,
}
#[derive(Debug, Serialize)]
pub struct Results {
    pub entries: Vec<Entry>,
    pub total: u32,
    pub offset: u32,
}

pub fn query(options: Query) -> Result<Results, String> {
    if options.query.len() > 8192 || options.query.contains('\0') {
        return Err("搜索条件过长或包含无效字符".into());
    }
    #[cfg(windows)]
    {
        native::query(options)
    }
    #[cfg(not(windows))]
    {
        let _ = options;
        Err("Everything 仅支持 Windows".into())
    }
}

pub fn status() -> serde_json::Value {
    #[cfg(windows)]
    {
        native::status()
    }
    #[cfg(not(windows))]
    {
        serde_json::json!({"running":false,"instances":[]})
    }
}

#[cfg(any(windows, test))]
fn parse(bytes: &[u8]) -> Result<Results, String> {
    fn u32_at(b: &[u8], n: usize) -> Result<u32, String> {
        b.get(n..n + 4)
            .and_then(|v| v.try_into().ok())
            .map(u32::from_le_bytes)
            .ok_or_else(|| "Everything 响应被截断".into())
    }
    fn u64_at(b: &[u8], n: usize) -> Result<u64, String> {
        b.get(n..n + 8)
            .and_then(|v| v.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or_else(|| "Everything 元数据被截断".into())
    }
    let total = u32_at(bytes, 0)?;
    let count = u32_at(bytes, 4)? as usize;
    let offset = u32_at(bytes, 8)?;
    if count > 1000 || bytes.len() < 20 + count * 8 || u32_at(bytes, 12)? != 0x54 {
        return Err("Everything 返回了不支持的结果格式".into());
    }
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let flags = u32_at(bytes, 20 + i * 8)?;
        let at = u32_at(bytes, 24 + i * 8)? as usize;
        if at < 20 + count * 8 {
            return Err("Everything 返回了无效数据偏移".into());
        }
        let length = u32_at(bytes, at)? as usize;
        if length > 32767 {
            return Err("Everything 返回路径过长".into());
        }
        let end = at + 4 + length * 2;
        let text = bytes.get(at + 4..end).ok_or("Everything 路径被截断")?;
        if bytes.get(end..end + 2) != Some(&[0, 0]) {
            return Err("Everything 路径缺少结束标记".into());
        }
        let utf16: Vec<u16> = text
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect();
        let path = String::from_utf16(&utf16).map_err(|_| "Everything 路径编码无效")?;
        let size = u64_at(bytes, end + 2)?;
        let timestamp = u64_at(bytes, end + 10)?;
        let name = path.rsplit(['\\', '/']).next().unwrap_or(&path).to_string();
        entries.push(Entry {
            path,
            name,
            directory: flags & 3 != 0,
            size: if size == u64::MAX { 0 } else { size },
            modified: timestamp.saturating_sub(116444736000000000) / 10000000,
        });
    }
    Ok(Results {
        entries,
        total,
        offset,
    })
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    #[repr(C)]
    struct CopyData {
        tag: usize,
        length: u32,
        data: *const u8,
    }
    struct Reply {
        token: usize,
        bytes: Option<Vec<u8>>,
    }
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    fn server(instance: &str) -> HWND {
        let class = if instance.is_empty() {
            "EVERYTHING_TASKBAR_NOTIFICATION".into()
        } else {
            format!("EVERYTHING_TASKBAR_NOTIFICATION_({instance})")
        };
        unsafe { FindWindowW(wide(&class).as_ptr(), std::ptr::null()) }
    }
    pub(super) fn status() -> serde_json::Value {
        let instances: Vec<_> = ["", "1.5a"]
            .into_iter()
            .filter(|name| !server(name).is_null())
            .collect();
        serde_json::json!({"running":!instances.is_empty(),"instances":instances})
    }
    unsafe extern "system" fn receive(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if message == WM_COPYDATA && lparam != 0 {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Reply;
            if !ptr.is_null() {
                let data = &*(lparam as *const CopyData);
                if data.tag == (*ptr).token
                    && data.length <= 16 * 1024 * 1024
                    && !data.data.is_null()
                {
                    (*ptr).bytes =
                        Some(std::slice::from_raw_parts(data.data, data.length as usize).to_vec());
                    return 1;
                }
            }
        }
        DefWindowProcW(hwnd, message, wparam, lparam)
    }
    pub(super) fn query(options: Query) -> Result<Results, String> {
        let instance = options.instance.unwrap_or_default();
        if instance.len() > 80 || instance.contains(['\0', '(', ')']) {
            return Err("Everything 实例名称无效".into());
        }
        let mut target = server(&instance);
        if target.is_null() && instance.is_empty() {
            target = server("1.5a");
        }
        if target.is_null() {
            return Err("未连接到 Everything。请启动已安装的 Everything，并等待索引完成；支持 1.4 和 1.5a。".into());
        }
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("MyTools Everything query").as_ptr(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            if hwnd.is_null() {
                return Err("无法创建 Everything 查询窗口".into());
            }
            use std::hash::{BuildHasher, Hasher};
            let token = std::collections::hash_map::RandomState::new()
                .build_hasher()
                .finish() as u32
                | 1;
            let mut reply = Box::new(Reply {
                token: token as usize,
                bytes: None,
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *reply as *mut Reply) as isize);
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, receive as *const () as isize);
            let mut query = Vec::new();
            for v in [
                hwnd as usize as u32,
                reply.token as u32,
                0,
                options.offset,
                options.limit.unwrap_or(100).clamp(1, 1000),
                0x54,
                options.sort.filter(|s| (1..=26).contains(s)).unwrap_or(1),
            ] {
                query.extend_from_slice(&v.to_le_bytes());
            }
            for ch in options.query.encode_utf16().chain(Some(0)) {
                query.extend_from_slice(&ch.to_le_bytes());
            }
            let data = CopyData {
                tag: 18,
                length: query.len() as u32,
                data: query.as_ptr(),
            };
            let mut accepted = 0usize;
            let sent = SendMessageTimeoutW(
                target,
                WM_COPYDATA,
                hwnd as usize,
                (&data as *const CopyData) as isize,
                SMTO_ABORTIFHUNG,
                2000,
                &mut accepted,
            );
            let deadline = Instant::now() + Duration::from_secs(5);
            while sent != 0 && accepted != 0 && reply.bytes.is_none() && Instant::now() < deadline {
                let mut msg: MSG = std::mem::zeroed();
                while PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                if reply.bytes.is_none() {
                    MsgWaitForMultipleObjects(0, std::ptr::null(), 0, 30, QS_ALLINPUT);
                }
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DestroyWindow(hwnd);
            if sent == 0 || accepted == 0 {
                return Err("Everything 未接受查询，请检查版本、索引状态与运行权限".into());
            }
            parse(
                &reply
                    .bytes
                    .ok_or("Everything 查询超时，请等待索引完成后重试")?,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore]
    fn queries_the_installed_everything_instance() {
        assert_eq!(status()["running"], true);
        let results = query(Query {
            query: "Everything.exe".into(),
            offset: 0,
            limit: Some(5),
            sort: Some(1),
            instance: None,
        })
        .unwrap();
        assert!(results.total > 0);
        assert!(results
            .entries
            .iter()
            .all(|entry| entry.name.to_lowercase().ends_with("everything.exe")));
    }
    #[test]
    fn rejects_incomplete_or_malicious_results() {
        assert!(parse(&[]).is_err());
        let mut bytes = vec![0u8; 28];
        bytes[4..8].copy_from_slice(&1u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&0x54u32.to_le_bytes());
        bytes[24..28].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse(&bytes).is_err());
    }
    #[test]
    fn parses_unicode_result_and_filetime() {
        let path = "C:\\资料\\报告.md";
        let utf: Vec<_> = path.encode_utf16().collect();
        let mut b = Vec::new();
        for n in [1u32, 1, 0, 0x54, 1, 0, 28, utf.len() as u32] {
            b.extend_from_slice(&n.to_le_bytes());
        }
        for ch in utf {
            b.extend_from_slice(&ch.to_le_bytes());
        }
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(&42u64.to_le_bytes());
        b.extend_from_slice(&116444736010000000u64.to_le_bytes());
        let r = parse(&b).unwrap();
        assert_eq!(r.entries[0].name, "报告.md");
        assert_eq!(r.entries[0].size, 42);
        assert_eq!(r.entries[0].modified, 1);
    }
}
