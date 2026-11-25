use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub maximized: bool,
}

pub fn fit(
    saved: &Bounds,
    monitor: (i32, i32, u32, u32),
    scale: f64,
    minimum: (u32, u32),
) -> Bounds {
    let old = if saved.scale.is_finite() && saved.scale > 0.0 {
        saved.scale
    } else {
        1.0
    };
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let width = ((f64::from(saved.width) / old * scale).round() as u32)
        .max(minimum.0)
        .min(monitor.2.max(1));
    let height = ((f64::from(saved.height) / old * scale).round() as u32)
        .max(minimum.1)
        .min(monitor.3.max(1));
    Bounds {
        x: saved.x.clamp(
            monitor.0,
            monitor
                .0
                .saturating_add(monitor.2.saturating_sub(width) as i32),
        ),
        y: saved.y.clamp(
            monitor.1,
            monitor
                .1
                .saturating_add(monitor.3.saturating_sub(height) as i32),
        ),
        width,
        height,
        scale,
        maximized: saved.maximized,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restores_logical_size_after_dpi_change_and_keeps_window_on_screen() {
        let saved = Bounds {
            x: 9000,
            y: -2000,
            width: 1200,
            height: 900,
            scale: 1.5,
            maximized: true,
        };
        let fitted = fit(&saved, (0, 0, 1920, 1040), 1.0, (480, 320));
        assert_eq!((fitted.width, fitted.height), (800, 600));
        assert_eq!((fitted.x, fitted.y), (1120, 0));
        assert!(fitted.maximized);
        let small = fit(&saved, (-800, 0, 800, 400), 2.0, (1024, 760));
        assert_eq!(
            (small.x, small.y, small.width, small.height),
            (-800, 0, 800, 400)
        );
    }
}
