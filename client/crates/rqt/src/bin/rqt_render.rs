use rqt::gui::{QColor, QPainter, QRectF};
use rqt::paint::paint_widget_tree;
use rqt::raster::rasterize;
use rqt::style::QFusionStyle;
use rqt::widgets::load_ui;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let form_path = args.next().ok_or("usage: rqt_render <form.ui> <out.png>")?;
    let out_path = args.next().ok_or("usage: rqt_render <form.ui> <out.png>")?;
    let xml = std::fs::read_to_string(&form_path).map_err(|error| error.to_string())?;
    let (mut tree, root) = load_ui(&xml)?;
    // Qt clamps a window to its layout minimum on show (and the app does
    // the same at createForm); without this, forms with long texts render
    // narrower in review than in Qt and the diff reports size 999.
    let min = tree.minimum_size_hint(root);
    let geometry = tree.widget(root).ok_or("missing root widget")?.geometry;
    let width = geometry.width.max(min.width).ceil().max(1.0) as u32;
    let height = geometry.height.max(min.height).ceil().max(1.0) as u32;
    // Relayout at the clamped size: `load_ui` laid the children out at the
    // .ui geometry, so a window widened to the minimum kept the narrow
    // geometry and clipped long labels (gallery/100, /105, /121: Qt shows
    // the full line, we cut it). Qt lays out when the widget is shown, i.e.
    // at the final size.
    tree.layout(root, QRectF::new(0.0, 0.0, width as f32, height as f32));

    // Gallery A/B goldens are idle by default. Opt-in focused snapshot:
    // a widget whose objectName is `focus` (see gallery/focus/*).
    if let Some(widget) = tree.find_widget("focus") {
        if tree.set_focus(widget) {
            let class = tree
                .widget(widget)
                .map(|node| node.class_name.as_str())
                .unwrap_or("");
            if matches!(class, "QLineEdit" | "QPlainTextEdit") {
                let length = tree.text(widget).unwrap_or_default().chars().count();
                tree.set_cursor_position(widget, length);
            }
        }
    }
    let mut painter = QPainter::new();
    paint_widget_tree(&tree, root, &QFusionStyle::default(), &mut painter);
    let image = rasterize(
        painter.commands(),
        width,
        height,
        QColor::rgb(0xef, 0xef, 0xef),
    );
    image.save_png(out_path)
}
