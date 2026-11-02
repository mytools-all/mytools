use std::{collections::HashMap, path::PathBuf, sync::Mutex, time::Instant};

#[derive(Default)]
pub struct DroppedFiles(Mutex<HashMap<String, DropRecord>>);

type DropRecord = (Instant, Vec<PathBuf>, bool);

impl DroppedFiles {
    pub fn record(&self, window: &str, paths: Vec<PathBuf>) -> bool {
        if let Ok(mut entries) = self.0.lock() {
            if entries.get(window).is_some_and(|(time, previous, _)| time.elapsed().as_millis() < 500 && previous == &paths) { return false; }
            entries.retain(|_, (time, _, _)| time.elapsed().as_secs() < 60);
            entries.insert(window.into(), (Instant::now(), paths.into_iter().take(1000).collect(), false));
            return true;
        }
        false
    }

    pub fn take(&self, window: &str) -> Result<Vec<PathBuf>, String> {
        let mut entries = self.0.lock().map_err(|_| "拖放状态不可用".to_string())?;
        let entry = entries.get_mut(window).filter(|(time, _, consumed)| time.elapsed().as_secs() < 60 && !*consumed)
            .ok_or("没有有效的系统拖放文件，请重新拖入")?;
        entry.2 = true;
        Ok(entry.1.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_drop_is_window_bound_and_consumed_once() {
        let files = DroppedFiles::default();
        files.record("main", vec![PathBuf::from("fixture.txt")]);
        assert!(files.take("other").is_err());
        assert_eq!(files.take("main").unwrap(), vec![PathBuf::from("fixture.txt")]);
        assert!(!files.record("main", vec![PathBuf::from("fixture.txt")]));
        assert!(files.take("main").is_err());
    }
}
