use crate::gui::{QRectF, QSizeF};
use form_xml::PropValue;

pub type QObjectId = usize;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum QVariant {
    #[default]
    Invalid,
    Bool(bool),
    Int(i64),
    Double(f64),
    String(String),
    Enum(String),
    List(Vec<QVariant>),
    Rect(QRectF),
    Size(QSizeF),
    Map(Vec<(String, QVariant)>),
}

impl QVariant {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn from_prop(value: &PropValue) -> Self {
        use PropValue::*;
        match value {
            Str(value) => Self::String(value.clone()),
            Int(value) => Self::Int(*value),
            Bool(value) => Self::Bool(*value),
            Enum(value) => Self::Enum(value.clone()),
            Rect { x, y, w, h } => {
                Self::Rect(QRectF::new(*x as f32, *y as f32, *w as f32, *h as f32))
            }
            Size { w, h } => Self::Size(QSizeF::new(*w as f32, *h as f32)),
            List(values) => Self::List(values.iter().map(Self::from_prop).collect()),
            Map(entries) => Self::Map(
                entries
                    .iter()
                    .map(|(key, value)| (key.clone(), Self::from_prop(value)))
                    .collect(),
            ),
        }
    }

    pub fn to_prop(&self) -> Option<PropValue> {
        Some(match self {
            Self::Invalid => return None,
            Self::Bool(value) => PropValue::Bool(*value),
            Self::Int(value) => PropValue::Int(*value),
            Self::Double(value) => PropValue::Int(*value as i64),
            Self::String(value) => PropValue::Str(value.clone()),
            Self::Enum(value) => PropValue::Enum(value.clone()),
            Self::List(values) => {
                PropValue::List(values.iter().filter_map(QVariant::to_prop).collect())
            }
            Self::Rect(rect) => PropValue::Rect {
                x: rect.x as i64,
                y: rect.y as i64,
                w: rect.width as i64,
                h: rect.height as i64,
            },
            Self::Size(size) => PropValue::Size {
                w: size.width as i64,
                h: size.height as i64,
            },
            Self::Map(entries) => PropValue::Map(
                entries
                    .iter()
                    .filter_map(|(key, value)| value.to_prop().map(|value| (key.clone(), value)))
                    .collect(),
            ),
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QEventType {
    #[default]
    None,
    MouseButtonPress,
    MouseButtonRelease,
    MouseButtonDblClick,
    KeyPress,
    KeyRelease,
    FocusIn,
    FocusOut,
    Enter,
    Leave,
    Resize,
    Close,
    Paint,
}

impl QEventType {
    pub fn qt_name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::MouseButtonPress => "MouseButtonPress",
            Self::MouseButtonRelease => "MouseButtonRelease",
            Self::MouseButtonDblClick => "MouseButtonDblClick",
            Self::KeyPress => "KeyPress",
            Self::KeyRelease => "KeyRelease",
            Self::FocusIn => "FocusIn",
            Self::FocusOut => "FocusOut",
            Self::Enter => "Enter",
            Self::Leave => "Leave",
            Self::Resize => "Resize",
            Self::Close => "Close",
            Self::Paint => "Paint",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QEvent {
    pub event_type: QEventType,
    pub accepted: bool,
}

impl QEvent {
    pub fn new(event_type: QEventType) -> Self {
        Self {
            event_type,
            accepted: false,
        }
    }

    pub fn accept(&mut self) {
        self.accepted = true;
    }

    pub fn ignore(&mut self) {
        self.accepted = false;
    }

    pub fn is_accepted(&self) -> bool {
        self.accepted
    }
}

#[derive(Clone, Debug)]
pub struct QObject {
    pub object_name: String,
    pub parent: Option<QObjectId>,
    pub children: Vec<QObjectId>,
    pub properties: Vec<(String, QVariant)>,
    pub enabled: bool,
}

impl QObject {
    fn new(object_name: String) -> Self {
        Self {
            object_name,
            parent: None,
            children: Vec::new(),
            properties: Vec::new(),
            enabled: true,
        }
    }

    pub fn property(&self, name: &str) -> Option<&QVariant> {
        self.properties
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    pub fn set_property(&mut self, name: impl Into<String>, value: QVariant) {
        let name = name.into();
        if let Some(entry) = self.properties.iter_mut().find(|(key, _)| key == &name) {
            entry.1 = value;
        } else {
            self.properties.push((name, value));
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct QObjectTree {
    objects: Vec<QObject>,
}

impl QObjectTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_object(&mut self, object_name: impl Into<String>) -> QObjectId {
        let id = self.objects.len();
        self.objects.push(QObject::new(object_name.into()));
        id
    }

    pub fn add_child(&mut self, parent: QObjectId, child: QObjectId) -> bool {
        if parent == child
            || parent >= self.objects.len()
            || child >= self.objects.len()
            || self.is_ancestor(child, parent)
        {
            return false;
        }
        if let Some(previous) = self.objects[child].parent {
            if let Some(previous_parent) = self.objects.get_mut(previous) {
                previous_parent.children.retain(|id| id != &child);
            }
        }
        self.objects[child].parent = Some(parent);
        if let Some(parent) = self.objects.get_mut(parent) {
            parent.children.push(child);
        }
        true
    }

    pub fn is_ancestor(&self, ancestor: QObjectId, object: QObjectId) -> bool {
        let mut current = Some(object);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.objects.get(id).and_then(|object| object.parent);
        }
        false
    }

    pub fn object(&self, id: QObjectId) -> Option<&QObject> {
        self.objects.get(id)
    }

    pub fn object_mut(&mut self, id: QObjectId) -> Option<&mut QObject> {
        self.objects.get_mut(id)
    }

    pub fn set_property(
        &mut self,
        id: QObjectId,
        name: impl Into<String>,
        value: QVariant,
    ) -> bool {
        let Some(object) = self.objects.get_mut(id) else {
            return false;
        };
        object.set_property(name, value);
        true
    }

    pub fn property(&self, id: QObjectId, name: &str) -> Option<&QVariant> {
        self.objects.get(id)?.property(name)
    }

    pub fn find_object(&self, object_name: &str) -> Option<QObjectId> {
        self.objects
            .iter()
            .position(|object| object.object_name == object_name)
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use form_xml::PropValue;

    #[test]
    fn object_tree_links_parent_and_children() {
        let mut tree = QObjectTree::new();
        let root = tree.create_object("Dialog");
        let label = tree.create_object("label_1");
        let line = tree.create_object("lineEdit_1");

        assert!(tree.add_child(root, label));
        assert!(tree.add_child(root, line));
        assert_eq!(tree.object(root).unwrap().children, vec![label, line]);
        assert_eq!(tree.object(label).unwrap().parent, Some(root));
        assert!(tree.is_ancestor(root, line));
        assert!(!tree.is_ancestor(line, root));

        assert!(tree.add_child(label, root) == false);
        assert!(tree.add_child(label, label) == false);
    }

    #[test]
    fn object_properties_are_qt_variants() {
        let mut tree = QObjectTree::new();
        let object = tree.create_object("label_1");

        assert!(tree.set_property(object, "text", QVariant::String("A=".into())));
        assert!(tree.set_property(object, "enabled", QVariant::Bool(false)));

        assert_eq!(
            tree.property(object, "text"),
            Some(&QVariant::String("A=".into()))
        );
        assert_eq!(
            tree.property(object, "enabled").and_then(QVariant::as_bool),
            Some(false)
        );
        assert_eq!(tree.property(object, "missing"), None);
    }

    #[test]
    fn variant_roundtrips_form_properties() {
        let values = [
            PropValue::Str("text".into()),
            PropValue::Int(42),
            PropValue::Bool(true),
            PropValue::Enum("Qt::Horizontal".into()),
            PropValue::Rect {
                x: 1,
                y: 2,
                w: 3,
                h: 4,
            },
            PropValue::Size { w: 5, h: 6 },
            PropValue::List(vec![PropValue::Str("one".into()), PropValue::Int(2)]),
        ];

        for value in values {
            let variant = QVariant::from_prop(&value);
            let roundtrip = variant.to_prop().expect("converts back");
            match (&value, &roundtrip) {
                (PropValue::List(left), PropValue::List(right)) => {
                    assert_eq!(left.len(), right.len())
                }
                _ => assert_eq!(value, roundtrip),
            }
        }
    }

    #[test]
    fn event_names_match_qt() {
        assert_eq!(
            QEventType::MouseButtonRelease.qt_name(),
            "MouseButtonRelease"
        );
        assert_eq!(
            QEventType::MouseButtonDblClick.qt_name(),
            "MouseButtonDblClick"
        );
        assert_eq!(QEventType::Close.qt_name(), "Close");
    }
}
