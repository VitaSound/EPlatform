use rqt::gui::{QColor, QPainter};
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
    let root_widget = tree.widget(root).ok_or("missing root widget")?;
    let width = root_widget.geometry.width.ceil().max(1.0) as u32;
    let height = root_widget.geometry.height.ceil().max(1.0) as u32;

    tree.focus_first(root);
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
