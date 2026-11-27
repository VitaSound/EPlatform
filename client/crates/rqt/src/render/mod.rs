mod geometry;
mod lower;
mod primitive;
pub mod sdf;

pub use geometry::{
    inside_rounded_rect, rounded_horizontal_span_at_y, rounded_vertical_span_at_x,
};
pub use lower::{
    expand_dashed_stroke_rect, expand_linear_gradient, expand_stroke_rounded_border,
    lower_commands, LoweredRect,
};
pub use primitive::RenderPrimitive;
pub use sdf::{
    rounded_fill_cover, rounded_stroke_cover, sd_rounded_box, sd_rounded_rect, smoothstep,
};