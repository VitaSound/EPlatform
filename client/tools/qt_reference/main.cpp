#include <QImage>
#include <QLayout>
#include <QPainter>
#include <QApplication>
#include <QFile>
#include <QFont>
#include <QFontMetricsF>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QStyleFactory>
#include <QUiLoader>
#include <QWidget>

static QJsonObject dumpWidget(QWidget *widget, QWidget *root)
{
    const QPoint pos = widget->mapTo(root, QPoint(0, 0));
    QJsonObject object;
    object["class"] = QString::fromLatin1(widget->metaObject()->className());
    object["name"] = widget->objectName();
    object["x"] = pos.x();
    object["y"] = pos.y();
    object["w"] = widget->width();
    object["h"] = widget->height();
    object["hint_w"] = widget->sizeHint().width();
    object["hint_h"] = widget->sizeHint().height();
    object["min_w"] = widget->minimumSizeHint().width();
    object["min_h"] = widget->minimumSizeHint().height();
    const QSizePolicy policy = widget->sizePolicy();
    object["policy_h"] = static_cast<int>(policy.horizontalPolicy());
    object["policy_v"] = static_cast<int>(policy.verticalPolicy());
    return object;
}

static void dumpLayout(QLayout *layout, QJsonArray &array)
{
    for (int i = 0; i < layout->count(); ++i) {
        QLayoutItem *item = layout->itemAt(i);
        if (!item) continue;
        const QRect geometry = item->geometry();
        QJsonObject object;
        object["i"] = i;
        object["x"] = geometry.x();
        object["y"] = geometry.y();
        object["w"] = geometry.width();
        object["h"] = geometry.height();
        object["hint_w"] = item->sizeHint().width();
        object["hint_h"] = item->sizeHint().height();
        if (QWidget *widget = item->widget()) {
            object["widget"] = widget->objectName();
            object["class"] = QString::fromLatin1(widget->metaObject()->className());
        } else if (QLayout *child = item->layout()) {
            object["layout"] = QString::fromLatin1(child->metaObject()->className());
        }
        array.append(object);
    }
}

static void collect(QWidget *widget, QWidget *root, QJsonArray &array)
{
    if (!widget->objectName().isEmpty()) {
        array.append(dumpWidget(widget, root));
    }
    const auto children = widget->findChildren<QWidget *>(QString(), Qt::FindDirectChildrenOnly);
    for (QWidget *child : children) {
        collect(child, root, array);
    }
}

int main(int argc, char **argv)
{
    QApplication app(argc, argv);
    QApplication::setStyle(QStyleFactory::create("Fusion"));
    QFont font("DejaVu Sans");
    font.setPixelSize(14);
    app.setFont(font);

    if (argc < 2) {
        qCritical("usage: qt_reference <form.ui>");
        return 2;
    }

    QFile file(QString::fromLocal8Bit(argv[1]));
    if (!file.open(QIODevice::ReadOnly)) {
        qCritical("cannot open %s", argv[1]);
        return 2;
    }

    QUiLoader loader;
    QWidget *root = loader.load(&file, nullptr);
    file.close();
    if (!root) {
        qCritical("cannot load form");
        return 2;
    }
    root->show();
    app.processEvents();

    QJsonObject output;
    output["form_class"] = QString::fromLatin1(root->metaObject()->className());
    output["form_name"] = root->objectName();
    output["geometry"] = QJsonObject{{"x", root->x()}, {"y", root->y()},
                                     {"w", root->width()}, {"h", root->height()}};
    QFontMetricsF metrics(app.font());
    output["font"] = QJsonObject{{"family", app.font().family()},
                                 {"pixel_size", app.font().pixelSize()},
                                 {"ascent", metrics.ascent()},
                                 {"descent", metrics.descent()},
                                 {"height", metrics.height()},
                                 {"x_advance", metrics.horizontalAdvance('x')},
                                 {"m_advance", metrics.horizontalAdvance('M')},
                                 {"textlabel_advance", metrics.horizontalAdvance("TextLabel")}};
    if (QLayout *layout = root->layout()) {
        const QMargins margins = layout->contentsMargins();
        QJsonObject layoutObject;
        layoutObject["left"] = margins.left();
        layoutObject["top"] = margins.top();
        layoutObject["right"] = margins.right();
        layoutObject["bottom"] = margins.bottom();
        layoutObject["spacing"] = layout->spacing();
        output["layout"] = layoutObject;
    }

    QJsonArray widgets;
    collect(root, root, widgets);
    output["widgets"] = widgets;
    if (QLayout *layout = root->layout()) {
        QJsonArray items;
        dumpLayout(layout, items);
        output["root_layout_items"] = items;
    }

    QJsonDocument document(output);
    QFile out;
    out.open(stdout, QIODevice::WriteOnly);
    out.write(document.toJson(QJsonDocument::Indented));

    if (argc >= 3) {
        QImage image(root->size(), QImage::Format_RGBA8888);
        image.fill(Qt::transparent);
        QPainter painter(&image);
        root->render(&painter, QPoint(), QRegion(),
                     QWidget::DrawWindowBackground | QWidget::DrawChildren);
        painter.end();
        if (!image.save(QString::fromLocal8Bit(argv[2]), "PNG")) {
            qCritical("cannot save %s", argv[2]);
            return 2;
        }
    }

    root->deleteLater();
    return 0;
}
