use crate::gui::{QColor, QPixmap, QRectF};

/// Intermediate render op after lowering paint commands (GPU batching follows).
#[derive(Clone, Debug, PartialEq)]
pub enum RenderPrimitive {
    SolidRect {
        rect: QRectF,
        /// `0` = axis-aligned fill; otherwise rounded fill.
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
    RoundedStroke {
        rect: QRectF,
        radius: f32,
        width: f32,
        color: QColor,
    },
    /// Axis-aligned dashed stroke (one GPU SDF quad / CPU dash segments).
    DashedStroke {
        rect: QRectF,
        width: f32,
        color: QColor,
        dash: f32,
        gap: f32,
    },
    Ellipse {
        rect: QRectF,
        /// `0` = filled; otherwise stroke width.
        stroke: f32,
        color: QColor,
    },
    Triangle {
        a: crate::gui::QPointF,
        b: crate::gui::QPointF,
        c: crate::gui::QPointF,
        color: QColor,
    },
    Pixmap {
        rect: QRectF,
        pixmap: QPixmap,
        opacity: f32,
    },
}
