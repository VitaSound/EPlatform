use crate::gui::{QColor, QRectF};

/// Intermediate render op after lowering paint commands (GPU batching follows).
#[derive(Clone, Debug, PartialEq)]
pub enum RenderPrimitive {
    SolidRect {
        rect: QRectF,
        /// `0` = axis-aligned fill; otherwise rounded fill expansion.
        radius: f32,
        color: QColor,
    },
    LinearGradient {
        rect: QRectF,
        radius: f32,
        start: QColor,
        end: QColor,
        horizontal: bool,
    },
}