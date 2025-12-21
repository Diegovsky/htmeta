use std::borrow::Cow;

use htmeta::{
    kdl::{KdlDocument, KdlNode, KdlValue},
    Error,
};

use easy_ext::ext;

pub fn err(msg: impl Into<String>) -> Error {
    Error::UserError {
        message: msg.into(),
    }
}

#[ext]
pub impl KdlDocument {
    fn remove_child(&mut self, key: &str) -> Option<KdlNode> {
        let i = self
            .nodes()
            .iter()
            .position(|node| node.name().value() == key)?;
        Some(self.nodes_mut().remove(i))
    }
}

pub enum Key<'a> {
    Arg(usize),
    Prop(&'a str),
}

impl<'a> From<Key<'a>> for Cow<'a, str> {
    fn from(value: Key<'a>) -> Self {
        match value {
            Key::Arg(id) => Cow::Owned(id.to_string()),
            Key::Prop(id) => Cow::Borrowed(id),
        }
    }
}

#[ext]
pub impl KdlNode {
    fn command_name(&self) -> Option<&str> {
        self.name().value().strip_prefix('@')
    }
    fn is_command(&self, name: &str) -> bool {
        self.command_name().map(|n| n == name).unwrap_or(false)
    }
    fn args(&self) -> impl Iterator<Item = &KdlValue> + DoubleEndedIterator {
        self.entries()
            .iter()
            .filter(|e| e.name().is_none())
            .map(|e| e.value())
    }

    // fn properties<'g>(&'g self) -> impl Iterator<Item = (&'g str, &'g KdlValue)> {
    //     self.entries()
    //         .iter()
    //         .filter_map(|e| Some((e.name()?.value(), e.value())))
    // }

    fn keyed_entries(&self) -> impl Iterator<Item = (Key, &KdlValue)> + DoubleEndedIterator {
        let mut current_index = 0;
        self.entries().iter().map(move |entry| match entry.name() {
            Some(key) => (Key::Prop(key.value()), entry.value()),
            None => {
                let id = current_index;
                current_index += 1;
                (Key::Arg(id), entry.value())
            }
        })
    }
}
