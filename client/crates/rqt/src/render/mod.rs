mod geometry;
mod lower;
mod primitive;

pub use geometry::{
    inside_rounded_rect, rounded_horizontal_span_at_y, rounded_vertical_span_at_x,
};
pub use lower::{expand_linear_gradient, expand_stroke_rounded_border, lower_commands, LoweredRect};
pub use primitive::RenderPrimitive;